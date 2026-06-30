//! Experiment execution.

use std::path::PathBuf;
use std::time::Duration;

use anyhow::{bail, Context, Result};

use crate::client::{
    default_target, finish_session, result_stats, run_paced_publish, run_sustained_load,
    spawn_connections, PublishOptions, ServerTarget,
};
use crate::experiments::{evaluate_step, DegradationThresholds, StepVerdict};
use crate::hardware::validate_hardware;
use crate::harness::capture_hardware;
use crate::report::BenchReport;
use crate::stats::MetricStats;

pub struct RunContext {
    pub experiment: String,
    pub hardware: String,
    pub server_url: String,
    pub server_urls: Vec<String>,
    pub connections: Option<u32>,
    pub rate_per_sec: Option<u32>,
    pub duration_secs: u32,
    pub payload_bytes: usize,
    pub allow_phase2: bool,
    pub substrate_report: Option<String>,
    pub report_path: Option<PathBuf>,
}

pub struct RunOutcome {
    pub report: BenchReport,
}

const PLS0_SWEEP: &[u32] = &[1, 4, 16, 64, 256, 512, 1024, 2048];
const PLS1_RATES: &[u32] = &[10, 100, 500, 1000, 2000, 5000, 10000];
const PLS2_SWEEP: &[u32] = &[1, 4, 16, 64, 128, 256];

pub async fn run_experiment(ctx: RunContext) -> Result<RunOutcome> {
    let profile = validate_hardware(&ctx.hardware, ctx.allow_phase2)?;
    std::env::set_var("PHOTON_LEPTOS_BENCH_HARDWARE", &ctx.hardware);

    let report = match ctx.experiment.as_str() {
        "bm-pls0" => run_pls0(&ctx, profile.phase).await?,
        "bm-pls1" => run_pls1(&ctx, profile.phase).await?,
        "bm-pls2" => run_pls2(&ctx, profile.phase).await?,
        "bm-pls3" => run_pls3(&ctx, profile.phase).await?,
        "bm-pls4" => run_pls4(&ctx, profile.phase).await?,
        "bm-pls6" => run_pls6(&ctx, profile.phase).await?,
        "bm-pls7" => run_pls7(&ctx, profile.phase).await?,
        "bm-pls8" => run_pls8(&ctx, profile.phase).await?,
        "bm-pls9" => run_pls9(&ctx, profile.phase).await?,
        other => bail!("unknown experiment: {other}"),
    };

    if let Some(path) = &ctx.report_path {
        report.write_json(path)?;
    }
    Ok(RunOutcome { report })
}

fn targets(ctx: &RunContext) -> Vec<ServerTarget> {
    if ctx.server_urls.is_empty() {
        vec![default_target(&ctx.server_url)]
    } else {
        ctx.server_urls
            .iter()
            .map(|u| default_target(u))
            .collect()
    }
}

fn base_report(ctx: &RunContext, phase: u32, scenario: &str) -> BenchReport {
    BenchReport {
        experiment: ctx.experiment.clone(),
        matrix_slug: "sqlite-embedded-off-embedded-composite".into(),
        scenario_id: scenario.into(),
        hardware: ctx.hardware.clone(),
        profile_phase: phase,
        backend_id: "embedded".into(),
        topology: "embedded-composite".into(),
        telemetry: "off".into(),
        storage: "sqlite".into(),
        ws_connection_count: ctx.connections,
        publish_rate_per_sec: ctx.rate_per_sec,
        payload_bytes: Some(ctx.payload_bytes),
        client_type: Some("synthetic".into()),
        ws_delivery_ms: None,
        connect_latency_ms: None,
        refetch_ms: None,
        achieved_ops_per_sec: None,
        error_rate: None,
        connect_fail_rate: None,
        knee_connection_count: None,
        server_urls: if ctx.server_urls.is_empty() {
            None
        } else {
            Some(ctx.server_urls.clone())
        },
        substrate_report: ctx.substrate_report.clone(),
        pass: false,
        status: "fail",
        error: None,
        hardware_detail: capture_hardware().ok(),
    }
}

async fn run_pls0(ctx: &RunContext, phase: u32) -> Result<BenchReport> {
    let rates = [100u32, 1000];
    let thresholds = DegradationThresholds::default();
    let target = default_target(&ctx.server_url);
    let mut knee = 0u32;

    for rate in rates {
        let mut last_pass = 0u32;
        for &n in PLS0_SWEEP {
            let result = run_sustained_load(
                &[target.clone()],
                n,
                &PublishOptions {
                    rate_per_sec: rate,
                    duration_secs: ctx.duration_secs.min(60),
                    payload_bytes: ctx.payload_bytes,
                },
                2,
            )
            .await
            .with_context(|| format!("pls0 n={n} rate={rate}"))?;
            let (delivery, connect, connect_fail, err) = result_stats(&result);
            if evaluate_step(&delivery, err, connect_fail, &thresholds) == StepVerdict::Pass {
                last_pass = n;
            } else {
                break;
            }
            let _ = connect;
        }
        knee = knee.max(last_pass);
    }

    let mut report = base_report(ctx, phase, "pls0-connection-sweep");
    report.knee_connection_count = Some(knee);
    report.pass = knee > 0;
    report.status = if report.pass { "pass" } else { "fail" };
    Ok(report)
}

async fn run_pls1(ctx: &RunContext, phase: u32) -> Result<BenchReport> {
    let n = ctx.connections.unwrap_or(64);
    let thresholds = DegradationThresholds::default();
    let target = default_target(&ctx.server_url);
    let mut max_rate = 0u32;

    for &rate in PLS1_RATES {
        let result = run_sustained_load(
            &[target.clone()],
            n,
            &PublishOptions {
                rate_per_sec: rate,
                duration_secs: ctx.duration_secs.min(60),
                payload_bytes: ctx.payload_bytes,
            },
            2,
        )
        .await?;
        let (delivery, _, connect_fail, err) = result_stats(&result);
        if evaluate_step(&delivery, err, connect_fail, &thresholds) == StepVerdict::Pass {
            max_rate = rate;
        } else {
            break;
        }
    }

    let mut report = base_report(ctx, phase, "pls1-rate-matrix");
    report.ws_connection_count = Some(n);
    report.achieved_ops_per_sec = Some(max_rate as f64);
    report.pass = max_rate > 0;
    report.status = if report.pass { "pass" } else { "fail" };
    Ok(report)
}

async fn run_pls2(ctx: &RunContext, phase: u32) -> Result<BenchReport> {
    let target = default_target(&ctx.server_url);
    let thresholds = DegradationThresholds::default();
    let mut max_m = 0u32;

    for &m in PLS2_SWEEP {
        let result = run_sustained_load(
            &[target.clone()],
            m,
            &PublishOptions {
                rate_per_sec: 100,
                duration_secs: 30,
                payload_bytes: ctx.payload_bytes,
            },
            2,
        )
        .await?;
        let (delivery, _, connect_fail, err) = result_stats(&result);
        if evaluate_step(&delivery, err, connect_fail, &thresholds) == StepVerdict::Pass
            && result.connected >= m
        {
            max_m = m;
        } else {
            break;
        }
    }

    let mut report = base_report(ctx, phase, "pls2-multi-connection");
    report.ws_connection_count = Some(max_m);
    report.pass = max_m > 0;
    report.status = if report.pass { "pass" } else { "fail" };
    Ok(report)
}

async fn run_pls3(ctx: &RunContext, phase: u32) -> Result<BenchReport> {
    let client = reqwest::Client::new();
    let target = default_target(&ctx.server_url);
    let n = ctx.connections.unwrap_or(64);
    let mut refetch_samples = Vec::new();

    let result = run_sustained_load(
        &[target.clone()],
        n,
        &PublishOptions {
            rate_per_sec: ctx.rate_per_sec.unwrap_or(100),
            duration_secs: ctx.duration_secs.min(30),
            payload_bytes: ctx.payload_bytes,
        },
        2,
    )
    .await?;

    for _ in 0..10 {
        let start = std::time::Instant::now();
        let url = format!("{}/api/bench/value", ctx.server_url.trim_end_matches('/'));
        let _ = client.get(&url).send().await?;
        refetch_samples.push(start.elapsed().as_secs_f64() * 1000.0);
    }

    let (delivery, _, connect_fail, err) = result_stats(&result);
    let mut report = base_report(ctx, phase, "pls3-refetch-tax");
    report.ws_connection_count = Some(n);
    report.ws_delivery_ms = Some(delivery);
    report.refetch_ms = Some(MetricStats::summarize(refetch_samples));
    report.connect_fail_rate = Some(connect_fail);
    report.error_rate = Some(err);
    report.pass = err < 0.001;
    report.status = if report.pass { "pass" } else { "fail" };
    Ok(report)
}

async fn run_pls4(ctx: &RunContext, phase: u32) -> Result<BenchReport> {
    let sizes = [64usize, 256, 1024, 4096];
    let target = default_target(&ctx.server_url);
    let n = ctx.connections.unwrap_or(64);
    let mut last_delivery = MetricStats::empty();

    for size in sizes {
        let result = run_sustained_load(
            &[target.clone()],
            n,
            &PublishOptions {
                rate_per_sec: 500,
                duration_secs: 30,
                payload_bytes: size,
            },
            2,
        )
        .await?;
        let (delivery, _, _, _) = result_stats(&result);
        last_delivery = delivery;
    }

    let mut report = base_report(ctx, phase, "pls4-payload-scaling");
    report.ws_connection_count = Some(n);
    report.ws_delivery_ms = Some(last_delivery);
    report.pass = last_delivery.count > 0;
    report.status = if report.pass { "pass" } else { "fail" };
    Ok(report)
}

async fn run_pls6(ctx: &RunContext, phase: u32) -> Result<BenchReport> {
    let n = ctx.connections.unwrap_or(64);
    let target = default_target(&ctx.server_url);
    let publish = PublishOptions {
        rate_per_sec: 100,
        duration_secs: 30,
        payload_bytes: ctx.payload_bytes,
    };

    let broadcast = run_sustained_load(
        &[target.clone()],
        n,
        &publish,
        2,
    )
    .await?;
    let (b_delivery, _, _, _) = result_stats(&broadcast);

    let mut keyed_target = target.clone();
    keyed_target.ws_path = "/ws/bench".into();
    let mut keyed_combined = crate::client::LoadGenResult::default();
    for i in 0..n {
        let session = spawn_connections(
            &keyed_target,
            &crate::client::ConnectOptions {
                count: 1,
                key_filter: Some(format!("key-{i}")),
            },
        )
        .await?;
        keyed_combined.connected += 1;
        finish_session(session).await;
    }
    run_paced_publish(&target, &publish).await?;
    tokio::time::sleep(Duration::from_secs(5)).await;

    let mut report = base_report(ctx, phase, "pls6-keyed-vs-broadcast");
    report.ws_connection_count = Some(n);
    report.ws_delivery_ms = Some(b_delivery);
    report.pass = broadcast.messages_received > 0;
    report.status = if report.pass { "pass" } else { "fail" };
    Ok(report)
}

async fn run_pls7(ctx: &RunContext, phase: u32) -> Result<BenchReport> {
    let n = ctx.connections.unwrap_or(100).min(1000);
    let target = default_target(&ctx.server_url);
    let session = spawn_connections(
        &target,
        &crate::client::ConnectOptions {
            count: n,
            key_filter: None,
        },
    )
    .await?;
    finish_session(session).await;

    let after = spawn_connections(
        &target,
        &crate::client::ConnectOptions {
            count: n,
            key_filter: None,
        },
    )
    .await?;
    let partial = finish_session(after).await;

    let mut report = base_report(ctx, phase, "pls7-reconnect-storm");
    report.ws_connection_count = Some(n);
    report.connect_fail_rate = Some(if n == 0 {
        1.0
    } else {
        partial.connect_failures as f64 / n as f64
    });
    report.pass = partial.connected >= n / 2;
    report.status = if report.pass { "pass" } else { "fail" };
    Ok(report)
}

async fn run_pls8(ctx: &RunContext, phase: u32) -> Result<BenchReport> {
    let n = ctx.connections.unwrap_or(64);
    let target = default_target(&ctx.server_url);
    let duration = ctx.duration_secs.min(300);
    let result = run_sustained_load(
        &[target],
        n,
        &PublishOptions {
            rate_per_sec: 500,
            duration_secs: duration,
            payload_bytes: ctx.payload_bytes,
        },
        2,
    )
    .await?;
    let (delivery, _, connect_fail, err) = result_stats(&result);

    let mut report = base_report(ctx, phase, "pls8-soak");
    report.ws_connection_count = Some(n);
    report.ws_delivery_ms = Some(delivery);
    report.connect_fail_rate = Some(connect_fail);
    report.error_rate = Some(err);
    report.pass = err < 0.01 && connect_fail < 0.05;
    report.status = if report.pass { "pass" } else { "fail" };
    Ok(report)
}

async fn run_pls9(ctx: &RunContext, phase: u32) -> Result<BenchReport> {
    let urls = targets(ctx);
    if urls.len() < 2 {
        bail!("bm-pls9 requires --server-urls with at least 2 entries (ALB targets)");
    }
    let n = ctx.connections.unwrap_or(256);
    let result = run_sustained_load(
        &urls,
        n,
        &PublishOptions {
            rate_per_sec: ctx.rate_per_sec.unwrap_or(100),
            duration_secs: ctx.duration_secs.min(60),
            payload_bytes: ctx.payload_bytes,
        },
        2,
    )
    .await?;
    let (delivery, _, connect_fail, err) = result_stats(&result);

    let mut report = base_report(ctx, phase, "pls9-alb-horizontal");
    report.ws_connection_count = Some(n);
    report.ws_delivery_ms = Some(delivery);
    report.connect_fail_rate = Some(connect_fail);
    report.error_rate = Some(err);
    report.server_urls = Some(
        urls.iter()
            .map(|t| t.base_http.clone())
            .collect(),
    );
    report.pass = err < 0.001 && connect_fail == 0.0;
    report.status = if report.pass { "pass" } else { "fail" };
    Ok(report)
}

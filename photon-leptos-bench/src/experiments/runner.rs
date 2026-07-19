//! Experiment execution.

use std::path::PathBuf;
use std::time::Duration;

use anyhow::{bail, Context, Result};

use crate::client::{
    default_target, ensure_ws_mode, finish_session, result_stats, run_keyed_load,
    run_sustained_load, spawn_connections, wait_for_health, PublishOptions, ServerTarget,
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
    /// Requested fanout mode (`per_subscribe` | `broadcast_hub`). Applied via API.
    pub ws_mode: Option<String>,
    pub key_groups: Option<u32>,
}

pub struct RunOutcome {
    pub report: BenchReport,
}

// In-process steps stay ≤512: tearing down ≥1024 tungstenite tasks has wedged
// the Tokio runtime on 2-vCPU loadgens. Higher N uses process-per-N probes.
const PLS0_INPROCESS_SWEEP: &[u32] = &[1, 4, 16, 64, 256, 512];
const PLS0_PROCESS_SWEEP: &[u32] = &[768, 1024, 1536, 2048, 3072, 4096, 6144, 8192];
const PLS1_RATES: &[u32] = &[10, 100, 500, 1000, 2000, 5000, 10000];
const PLS2_SWEEP: &[u32] = &[1, 4, 16, 64, 128, 256];
const PLS5_GROUPS: &[u32] = &[1, 4, 16, 64, 256];
const ACHIEVED_RATE_RATIO: f64 = 0.95;

pub async fn run_experiment(ctx: RunContext) -> Result<RunOutcome> {
    let profile = validate_hardware(&ctx.hardware, ctx.allow_phase2)?;
    std::env::set_var("PHOTON_LEPTOS_BENCH_HARDWARE", &ctx.hardware);

    let experiment = ctx.experiment.to_ascii_lowercase();
    let ws_mode = resolve_ws_mode(&experiment, ctx.ws_mode.as_deref());
    if let Some(ref mode) = ws_mode {
        let mode_urls: Vec<&str> = if !ctx.server_urls.is_empty() {
            ctx.server_urls.iter().map(String::as_str).collect()
        } else {
            vec![ctx.server_url.as_str()]
        };
        for url in mode_urls {
            ensure_ws_mode(url, mode)
                .await
                .with_context(|| format!("ensure ws mode {mode} on {url}"))?;
        }
    }

    let report = match experiment.as_str() {
        "bm-pls0" | "bm-pls0-hub" => run_pls0(&ctx, profile.phase, &ws_mode).await?,
        "bm-pls1" => run_pls1(&ctx, profile.phase, &ws_mode).await?,
        "bm-pls2" => run_pls2(&ctx, profile.phase, &ws_mode).await?,
        "bm-pls3" => run_pls3(&ctx, profile.phase, &ws_mode).await?,
        "bm-pls4" => run_pls4(&ctx, profile.phase, &ws_mode).await?,
        "bm-pls5" | "bm-pls5-hub" => run_pls5(&ctx, profile.phase, &ws_mode).await?,
        "bm-pls6" => run_pls6(&ctx, profile.phase, &ws_mode).await?,
        "bm-pls7" => run_pls7(&ctx, profile.phase, &ws_mode).await?,
        "bm-pls8" => run_pls8(&ctx, profile.phase, &ws_mode).await?,
        "bm-pls9" => run_pls9(&ctx, profile.phase, &ws_mode).await?,
        other => bail!("unknown experiment: {other}"),
    };

    if let Some(path) = &ctx.report_path {
        report.write_json(path)?;
    }
    Ok(RunOutcome { report })
}

fn resolve_ws_mode(experiment: &str, cli: Option<&str>) -> Option<String> {
    if experiment.ends_with("-hub") {
        return Some("broadcast_hub".into());
    }
    cli.map(str::to_string)
}

fn targets(ctx: &RunContext) -> Vec<ServerTarget> {
    if ctx.server_urls.is_empty() {
        vec![default_target(&ctx.server_url)]
    } else {
        ctx.server_urls.iter().map(|u| default_target(u)).collect()
    }
}

fn base_report(
    ctx: &RunContext,
    phase: u32,
    scenario: &str,
    ws_mode: &Option<String>,
) -> BenchReport {
    BenchReport {
        experiment: ctx.experiment.clone(),
        matrix_slug: "mem-embedded-off-embedded-composite".into(),
        scenario_id: scenario.into(),
        hardware: ctx.hardware.clone(),
        profile_phase: phase,
        backend_id: "embedded".into(),
        topology: "embedded-composite".into(),
        telemetry: "off".into(),
        storage: "mem".into(),
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
        max_pass_connection_count: None,
        knee_kind: None,
        ws_fanout_mode: ws_mode.clone(),
        key_group_count: ctx.key_groups,
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

async fn run_pls0(ctx: &RunContext, phase: u32, ws_mode: &Option<String>) -> Result<BenchReport> {
    let rates: Vec<u32> = match ctx.rate_per_sec {
        Some(r) => vec![r],
        None => vec![100, 1000],
    };
    let thresholds = DegradationThresholds::default();
    let target = default_target(&ctx.server_url);

    // One-shot probe when `--connections` is set (also used by process-per-N children).
    if let Some(n) = ctx.connections {
        let rate = rates[0];
        wait_for_health(&target, Duration::from_secs(60)).await?;
        let (pass, delivery, err, connect_fail, achieved) =
            pls0_step(ctx, &target, n, rate, &thresholds).await?;
        let mut report = base_report(ctx, phase, "pls0-connection-oneshot", ws_mode);
        report.ws_connection_count = Some(n);
        report.publish_rate_per_sec = Some(rate);
        report.ws_delivery_ms = Some(delivery);
        report.error_rate = Some(err);
        report.connect_fail_rate = Some(connect_fail);
        report.achieved_ops_per_sec = Some(achieved);
        report.pass = pass;
        report.status = if pass { "pass" } else { "fail" };
        if pass {
            report.max_pass_connection_count = Some(n);
            report.knee_kind = Some("lower_bound".into());
        }
        return Ok(report);
    }

    let mut best_last_pass = 0u32;
    let mut observed_fail = false;

    for rate in rates {
        if rate == 1000 {
            eprintln!("pls0 cooldown before rate=1000");
            let _ = tokio::task::spawn_blocking(|| {
                std::thread::sleep(Duration::from_secs(5));
            })
            .await;
            wait_for_health(&target, Duration::from_secs(60)).await?;
        } else {
            wait_for_health(&target, Duration::from_secs(60)).await?;
        }

        let mut last_pass = 0u32;
        let mut failed = false;

        for &n in PLS0_INPROCESS_SWEEP {
            eprintln!("pls0 step n={n} rate={rate} (in-process)");
            let (pass, delivery, err, connect_fail, _) =
                pls0_step(ctx, &target, n, rate, &thresholds).await?;
            eprintln!(
                "pls0 result n={n} rate={rate} p99={:?} err={err:.4} cfail={connect_fail:.4}",
                delivery.p99
            );
            if pass {
                last_pass = n;
            } else {
                failed = true;
                break;
            }
        }

        if !failed {
            for &n in PLS0_PROCESS_SWEEP {
                eprintln!("pls0 step n={n} rate={rate} (process-per-N)");
                let pass = pls0_process_probe(ctx, n, rate)?;
                eprintln!("pls0 child n={n} rate={rate} pass={pass}");
                if pass {
                    last_pass = n;
                } else {
                    failed = true;
                    break;
                }
            }
        }

        if failed {
            observed_fail = true;
        }
        best_last_pass = best_last_pass.max(last_pass);
        if !failed && last_pass == *PLS0_PROCESS_SWEEP.last().unwrap_or(&512) {
            // Hit max without FAIL for this rate.
        }
    }

    let mut report = base_report(ctx, phase, "pls0-connection-sweep", ws_mode);
    report.max_pass_connection_count = Some(best_last_pass);
    if observed_fail && best_last_pass > 0 {
        report.knee_connection_count = Some(best_last_pass);
        report.knee_kind = Some("observed".into());
        report.pass = true;
        report.status = "pass";
    } else if best_last_pass > 0 {
        report.knee_connection_count = None;
        report.knee_kind = Some("lower_bound".into());
        report.pass = true;
        report.status = "pass";
        report.error = Some(
            "PLS0 search hit max N without an observed FAIL; knee_connection_count omitted".into(),
        );
    } else {
        report.pass = false;
        report.status = "fail";
    }
    Ok(report)
}

async fn pls0_step(
    ctx: &RunContext,
    target: &ServerTarget,
    n: u32,
    rate: u32,
    thresholds: &DegradationThresholds,
) -> Result<(bool, MetricStats, f64, f64, f64)> {
    let duration = ctx.duration_secs.min(60);
    let result = run_sustained_load(
        std::slice::from_ref(target),
        n,
        &PublishOptions::simple(rate, duration, ctx.payload_bytes),
        2,
    )
    .await
    .with_context(|| format!("pls0 n={n} rate={rate}"))?;
    let (delivery, _connect, connect_fail, err) = result_stats(&result);
    let achieved = if duration == 0 {
        0.0
    } else {
        result.publishes_succeeded as f64 / f64::from(duration)
    };
    let rate_ok = achieved >= f64::from(rate) * ACHIEVED_RATE_RATIO;
    let pass = evaluate_step(&delivery, err, connect_fail, thresholds) == StepVerdict::Pass
        && rate_ok
        && result.connected == n;
    Ok((pass, delivery, err, connect_fail, achieved))
}

fn pls0_process_probe(ctx: &RunContext, n: u32, rate: u32) -> Result<bool> {
    let exe = std::env::current_exe().context("current_exe for pls0 process probe")?;
    let tmp = tempfile::Builder::new()
        .prefix("pls0-probe-")
        .suffix(".json")
        .tempfile()
        .context("temp report path")?;
    let report_path = tmp.path().to_path_buf();
    // Keep tempfile alive until we read it.
    let _keep = tmp;

    let mut cmd = std::process::Command::new(&exe);
    cmd.arg("run")
        .arg("--experiment")
        .arg(&ctx.experiment)
        .arg("--hardware")
        .arg(&ctx.hardware)
        .arg("--server-url")
        .arg(&ctx.server_url)
        .arg("--connections")
        .arg(n.to_string())
        .arg("--rate-per-sec")
        .arg(rate.to_string())
        .arg("--duration-secs")
        .arg(ctx.duration_secs.min(60).to_string())
        .arg("--payload-bytes")
        .arg(ctx.payload_bytes.to_string())
        .arg("--report")
        .arg(&report_path);
    if ctx.allow_phase2 {
        cmd.arg("--allow-phase2");
    }
    if let Some(ref mode) = ctx.ws_mode {
        cmd.arg("--ws-mode").arg(mode);
    }
    if !ctx.server_urls.is_empty() {
        cmd.arg("--server-urls").arg(ctx.server_urls.join(","));
    }

    let status = cmd.status().context("spawn pls0 child")?;
    if !status.success() {
        // Child may still have written a fail report.
    }
    let raw = std::fs::read_to_string(&report_path)
        .with_context(|| format!("read child report {}", report_path.display()))?;
    let v: serde_json::Value = serde_json::from_str(&raw).context("parse child report")?;
    Ok(v.get("pass").and_then(|p| p.as_bool()).unwrap_or(false))
}

async fn run_pls1(ctx: &RunContext, phase: u32, ws_mode: &Option<String>) -> Result<BenchReport> {
    let n = ctx.connections.unwrap_or(64);
    let thresholds = DegradationThresholds::default();
    let target = default_target(&ctx.server_url);
    let mut max_achieved = 0.0f64;
    let duration = ctx.duration_secs.min(60);

    for &rate in PLS1_RATES {
        let result = run_sustained_load(
            std::slice::from_ref(&target),
            n,
            &PublishOptions::simple(rate, duration, ctx.payload_bytes),
            2,
        )
        .await?;
        let (delivery, _, connect_fail, err) = result_stats(&result);
        let achieved = if duration == 0 {
            0.0
        } else {
            result.publishes_succeeded as f64 / f64::from(duration)
        };
        let rate_ok = achieved >= f64::from(rate) * ACHIEVED_RATE_RATIO;
        let pass = evaluate_step(&delivery, err, connect_fail, &thresholds) == StepVerdict::Pass
            && rate_ok
            && result.connected == n;
        eprintln!(
            "pls1 rate={rate} achieved={achieved:.1} p99={:?} err={err:.4} pass={pass}",
            delivery.p99
        );
        if pass {
            max_achieved = achieved;
        } else {
            break;
        }
    }

    let mut report = base_report(ctx, phase, "pls1-rate-matrix", ws_mode);
    report.ws_connection_count = Some(n);
    report.achieved_ops_per_sec = Some(max_achieved);
    report.pass = max_achieved > 0.0;
    report.status = if report.pass { "pass" } else { "fail" };
    Ok(report)
}

async fn run_pls2(ctx: &RunContext, phase: u32, ws_mode: &Option<String>) -> Result<BenchReport> {
    let target = default_target(&ctx.server_url);
    let thresholds = DegradationThresholds::default();
    let mut max_m = 0u32;

    for &m in PLS2_SWEEP {
        let result = run_sustained_load(
            std::slice::from_ref(&target),
            m,
            &PublishOptions::simple(100, 30, ctx.payload_bytes),
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

    let mut report = base_report(ctx, phase, "pls2-multi-connection", ws_mode);
    report.ws_connection_count = Some(max_m);
    report.pass = max_m > 0;
    report.status = if report.pass { "pass" } else { "fail" };
    Ok(report)
}

async fn run_pls3(ctx: &RunContext, phase: u32, ws_mode: &Option<String>) -> Result<BenchReport> {
    let client = reqwest::Client::new();
    let target = default_target(&ctx.server_url);
    let n = ctx.connections.unwrap_or(64);
    let mut refetch_samples = Vec::new();

    let result = run_sustained_load(
        std::slice::from_ref(&target),
        n,
        &PublishOptions::simple(
            ctx.rate_per_sec.unwrap_or(100),
            ctx.duration_secs.min(30),
            ctx.payload_bytes,
        ),
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
    let mut report = base_report(ctx, phase, "pls3-refetch-tax", ws_mode);
    report.ws_connection_count = Some(n);
    report.ws_delivery_ms = Some(delivery);
    report.refetch_ms = Some(MetricStats::summarize(refetch_samples));
    report.connect_fail_rate = Some(connect_fail);
    report.error_rate = Some(err);
    report.pass = err < 0.001;
    report.status = if report.pass { "pass" } else { "fail" };
    Ok(report)
}

async fn run_pls4(ctx: &RunContext, phase: u32, ws_mode: &Option<String>) -> Result<BenchReport> {
    let sizes = [64usize, 256, 1024, 4096];
    let target = default_target(&ctx.server_url);
    let n = ctx.connections.unwrap_or(64);
    let mut last_delivery = MetricStats::empty();

    for size in sizes {
        let result = run_sustained_load(
            std::slice::from_ref(&target),
            n,
            &PublishOptions::simple(500, 30, size),
            2,
        )
        .await?;
        let (delivery, _, _, _) = result_stats(&result);
        last_delivery = delivery;
    }

    let mut report = base_report(ctx, phase, "pls4-payload-scaling", ws_mode);
    report.ws_connection_count = Some(n);
    report.ws_delivery_ms = Some(last_delivery);
    report.pass = last_delivery.count > 0;
    report.status = if report.pass { "pass" } else { "fail" };
    Ok(report)
}

/// N clients × G distinct key filters — hub gains vs auth-scoped cardinality.
async fn run_pls5(ctx: &RunContext, phase: u32, ws_mode: &Option<String>) -> Result<BenchReport> {
    let n = ctx.connections.unwrap_or(256);
    let thresholds = DegradationThresholds::default();
    let target = default_target(&ctx.server_url);
    let groups: Vec<u32> = match ctx.key_groups {
        Some(g) => vec![g],
        None => PLS5_GROUPS.to_vec(),
    };

    let mut last_pass_g = 0u32;
    let mut last_delivery = MetricStats::empty();

    for &g in &groups {
        let g = g.min(n).max(1);
        let result = run_keyed_load(
            &target,
            n,
            g,
            &PublishOptions::simple(100, ctx.duration_secs.min(60), ctx.payload_bytes),
            2,
        )
        .await
        .with_context(|| format!("pls5 n={n} g={g}"))?;
        let (delivery, _, connect_fail, err) = result_stats(&result);
        last_delivery = delivery;
        if evaluate_step(&delivery, err, connect_fail, &thresholds) == StepVerdict::Pass {
            last_pass_g = g;
        } else {
            break;
        }
    }

    let mut report = base_report(ctx, phase, "pls5-key-working-set", ws_mode);
    report.ws_connection_count = Some(n);
    report.key_group_count = Some(last_pass_g);
    report.ws_delivery_ms = Some(last_delivery);
    report.pass = last_pass_g > 0;
    report.status = if report.pass { "pass" } else { "fail" };
    Ok(report)
}

async fn run_pls6(ctx: &RunContext, phase: u32, ws_mode: &Option<String>) -> Result<BenchReport> {
    let n = ctx.connections.unwrap_or(64);
    let thresholds = DegradationThresholds::default();
    let target = default_target(&ctx.server_url);
    let duration = ctx.duration_secs.min(30);

    let broadcast = run_sustained_load(
        std::slice::from_ref(&target),
        n,
        &PublishOptions::simple(100, duration, ctx.payload_bytes),
        2,
    )
    .await?;
    let (b_delivery, _, b_fail, b_err) = result_stats(&broadcast);
    let broadcast_pass = evaluate_step(&b_delivery, b_err, b_fail, &thresholds)
        == StepVerdict::Pass
        && broadcast.messages_received > 0;

    let keyed = run_keyed_load(
        &target,
        n,
        n,
        &PublishOptions::simple(100, duration, ctx.payload_bytes),
        2,
    )
    .await?;
    let (k_delivery, _, k_fail, k_err) = result_stats(&keyed);
    let keyed_pass = evaluate_step(&k_delivery, k_err, k_fail, &thresholds) == StepVerdict::Pass
        && keyed.messages_received > 0;

    let mut report = base_report(ctx, phase, "pls6-keyed-vs-broadcast", ws_mode);
    report.ws_connection_count = Some(n);
    report.key_group_count = Some(n);
    // Report keyed delivery as the harder path; broadcast must also pass.
    report.ws_delivery_ms = Some(k_delivery);
    report.pass = broadcast_pass && keyed_pass;
    report.status = if report.pass { "pass" } else { "fail" };
    if !broadcast_pass {
        report.error = Some("broadcast phase failed thresholds".into());
    } else if !keyed_pass {
        report.error = Some("keyed phase failed thresholds".into());
    }
    Ok(report)
}

async fn run_pls7(ctx: &RunContext, phase: u32, ws_mode: &Option<String>) -> Result<BenchReport> {
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

    let mut report = base_report(ctx, phase, "pls7-reconnect-storm", ws_mode);
    report.ws_connection_count = Some(n);
    report.connect_fail_rate = Some(if n == 0 {
        1.0
    } else {
        f64::from(partial.connect_failures) / f64::from(n)
    });
    report.pass = partial.connected >= n / 2;
    report.status = if report.pass { "pass" } else { "fail" };
    Ok(report)
}

async fn run_pls8(ctx: &RunContext, phase: u32, ws_mode: &Option<String>) -> Result<BenchReport> {
    let n = ctx.connections.unwrap_or(64);
    let target = default_target(&ctx.server_url);
    let duration = ctx.duration_secs.min(300);
    let result = run_sustained_load(
        &[target],
        n,
        &PublishOptions::simple(500, duration, ctx.payload_bytes),
        2,
    )
    .await?;
    let (delivery, _, connect_fail, err) = result_stats(&result);

    let mut report = base_report(ctx, phase, "pls8-soak", ws_mode);
    report.ws_connection_count = Some(n);
    report.ws_delivery_ms = Some(delivery);
    report.connect_fail_rate = Some(connect_fail);
    report.error_rate = Some(err);
    report.pass = err < 0.01 && connect_fail < 0.05;
    report.status = if report.pass { "pass" } else { "fail" };
    Ok(report)
}

async fn run_pls9(ctx: &RunContext, phase: u32, ws_mode: &Option<String>) -> Result<BenchReport> {
    let urls = targets(ctx);
    if urls.len() < 2 {
        bail!("bm-pls9 requires --server-urls with at least 2 entries (ALB targets)");
    }
    let n = ctx.connections.unwrap_or(256);
    let result = run_sustained_load(
        &urls,
        n,
        &PublishOptions::simple(
            ctx.rate_per_sec.unwrap_or(100),
            ctx.duration_secs.min(60),
            ctx.payload_bytes,
        ),
        2,
    )
    .await?;
    let (delivery, _, connect_fail, err) = result_stats(&result);

    let mut report = base_report(ctx, phase, "pls9-alb-horizontal", ws_mode);
    report.ws_connection_count = Some(n);
    report.ws_delivery_ms = Some(delivery);
    report.connect_fail_rate = Some(connect_fail);
    report.error_rate = Some(err);
    report.server_urls = Some(urls.iter().map(|t| t.base_http.clone()).collect());
    report.pass = err < 0.001 && connect_fail == 0.0;
    report.status = if report.pass { "pass" } else { "fail" };
    Ok(report)
}

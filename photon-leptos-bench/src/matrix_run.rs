//! Matrix campaign runner.

use std::path::PathBuf;

use anyhow::Result;

use crate::experiments::{run_experiment, RunContext};
use crate::matrix::{report_path, slice_experiments};

pub struct MatrixRunOptions {
    pub hardware: String,
    pub slice: String,
    pub server_url: String,
    pub reports_dir: PathBuf,
    pub duration_secs: u32,
    pub allow_phase2: bool,
    pub skip_existing: bool,
}

pub async fn run_matrix(opts: MatrixRunOptions) -> Result<()> {
    let experiments = slice_experiments(&opts.slice)?;
    for exp in experiments {
        let (experiment, hardware) = if let Some((e, h)) = exp.split_once(':') {
            (e.to_string(), h.to_string())
        } else {
            (exp, opts.hardware.clone())
        };
        let path = report_path(&opts.reports_dir, &experiment, &hardware);
        if opts.skip_existing && path.exists() {
            eprintln!("skip existing {}", path.display());
            continue;
        }
        eprintln!("run {experiment} hardware={hardware}");
        let outcome = run_experiment(RunContext {
            experiment,
            hardware,
            server_url: opts.server_url.clone(),
            server_urls: Vec::new(),
            connections: None,
            rate_per_sec: None,
            duration_secs: opts.duration_secs,
            payload_bytes: crate::DEFAULT_PAYLOAD_BYTES,
            allow_phase2: opts.allow_phase2,
            substrate_report: None,
            report_path: Some(path),
        })
        .await?;
        eprintln!(
            "{} status={} pass={}",
            outcome.report.experiment, outcome.report.status, outcome.report.pass
        );
    }
    Ok(())
}

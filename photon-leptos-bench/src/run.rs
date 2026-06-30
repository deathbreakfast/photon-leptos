//! Single experiment runner entry.

use std::path::PathBuf;

use anyhow::Result;

pub use crate::experiments::{run_experiment, RunContext, RunOutcome};

pub struct RunArgs {
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
    pub report: Option<PathBuf>,
}

pub async fn run_experiment_args(args: RunArgs) -> Result<RunOutcome> {
    run_experiment(RunContext {
        experiment: args.experiment,
        hardware: args.hardware,
        server_url: args.server_url,
        server_urls: args.server_urls,
        connections: args.connections,
        rate_per_sec: args.rate_per_sec,
        duration_secs: args.duration_secs,
        payload_bytes: args.payload_bytes,
        allow_phase2: args.allow_phase2,
        substrate_report: args.substrate_report,
        report_path: args.report,
    })
    .await
}

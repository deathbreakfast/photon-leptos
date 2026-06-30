//! Clap CLI for photon-leptos-bench.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::DEFAULT_PAYLOAD_BYTES;

#[derive(Parser)]
#[command(
    name = "photon-leptos-bench",
    about = "WebSocket + Leptos integration benchmark runner (BM-PLS*)"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// List registered experiment IDs.
    Experiments,
    /// Run one experiment against a running bench server.
    Run {
        #[arg(long, default_value = "bm-pls0")]
        experiment: String,
        #[arg(long, default_value = "dev-wsl")]
        hardware: String,
        #[arg(long, default_value = "http://127.0.0.1:8080")]
        server_url: String,
        #[arg(long, value_delimiter = ',')]
        server_urls: Vec<String>,
        #[arg(long)]
        connections: Option<u32>,
        #[arg(long)]
        rate_per_sec: Option<u32>,
        #[arg(long, default_value = "60")]
        duration_secs: u32,
        #[arg(long, default_value_t = DEFAULT_PAYLOAD_BYTES)]
        payload_bytes: usize,
        #[arg(long)]
        allow_phase2: bool,
        #[arg(long)]
        substrate_report: Option<String>,
        #[arg(long)]
        report: Option<PathBuf>,
    },
    /// Run a campaign slice.
    Matrix {
        #[arg(long, default_value = "dev-wsl")]
        hardware: String,
        #[arg(long, default_value = "pls-connection")]
        slice: String,
        #[arg(long, default_value = "http://127.0.0.1:8080")]
        server_url: String,
        #[arg(long, default_value = "photon-leptos-bench/reports")]
        reports_dir: PathBuf,
        #[arg(long, default_value = "60")]
        duration_secs: u32,
        #[arg(long)]
        allow_phase2: bool,
        #[arg(long)]
        skip_existing: bool,
    },
    /// Print hardware profile + live capture JSON.
    Hardware {
        #[arg(long, default_value = "aws-t3-medium")]
        profile: String,
        #[arg(long)]
        allow_phase2: bool,
    },
}

//! photon-leptos integration benchmarks (BM-PLS*).

pub mod cli;
pub mod client;
pub mod experiments;
pub mod hardware;
pub mod harness;
pub mod matrix;
pub mod matrix_run;
pub mod report;
pub mod run;
pub mod server;
pub mod stats;

pub const BENCH_TOPIC: &str = "bench.event";
pub const WS_PATH: &str = "/ws/bench";
pub const DEFAULT_PAYLOAD_BYTES: usize = 256;

//! Report schema for BM-PLS* experiments.

use std::path::Path;

use anyhow::Result;
use serde::Serialize;

use crate::harness::HardwareDetail;
use crate::stats::MetricStats;

#[derive(Debug, Serialize)]
pub struct BenchReport {
    pub experiment: String,
    pub matrix_slug: String,
    pub scenario_id: String,
    pub hardware: String,
    pub profile_phase: u32,
    pub backend_id: String,
    pub topology: String,
    pub telemetry: String,
    pub storage: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ws_connection_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publish_rate_per_sec: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_bytes: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ws_delivery_ms: Option<MetricStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connect_latency_ms: Option<MetricStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refetch_ms: Option<MetricStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub achieved_ops_per_sec: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_rate: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connect_fail_rate: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub knee_connection_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_urls: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub substrate_report: Option<String>,
    pub pass: bool,
    pub status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hardware_detail: Option<HardwareDetail>,
}

impl BenchReport {
    pub fn hardware_env() -> String {
        std::env::var("PHOTON_LEPTOS_BENCH_HARDWARE")
            .or_else(|_| std::env::var("PHOTON_BENCH_HARDWARE"))
            .unwrap_or_else(|_| "dev-wsl".into())
    }

    pub fn write_json(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }
}

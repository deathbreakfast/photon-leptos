//! Error types for photon-leptos.

use thiserror::Error;

/// Errors produced by photon-leptos.
#[derive(Error, Debug)]
pub enum PhotonLeptosError {
    /// WebSocket connection failed.
    #[error("WebSocket error: {0}")]
    WebSocket(String),

    /// Failed to serialize or deserialize JSON.
    #[error("JSON error: {0}")]
    Json(String),

    /// Server function invocation failed.
    #[error("Server function error: {0}")]
    ServerFn(String),
}

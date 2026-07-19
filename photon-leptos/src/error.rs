//! Error types surfaced by photon-leptos client helpers.

#![warn(missing_docs)]

use thiserror::Error;

/// Errors produced by photon-leptos client-side helpers.
#[derive(Error, Debug)]
pub enum PhotonLeptosError {
    /// WebSocket connection or message handling failed.
    #[error("WebSocket error: {0}")]
    WebSocket(String),

    /// Failed to serialize or deserialize JSON event payloads.
    #[error("JSON error: {0}")]
    Json(String),

    /// Underlying Leptos server function returned an error.
    #[error("Server function error: {0}")]
    ServerFn(String),
}

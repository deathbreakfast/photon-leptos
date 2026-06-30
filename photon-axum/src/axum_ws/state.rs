//! App state traits for Photon access in WebSocket handlers.

use std::sync::Arc;

use photon_runtime::Photon;

/// Implemented by Axum app state to provide `Arc<Photon>` to WebSocket handlers.
pub trait HasPhoton: Clone + Send + Sync + 'static {
    /// Return a shared handle to the running Photon runtime.
    fn photon_arc(&self) -> Arc<Photon>;
}

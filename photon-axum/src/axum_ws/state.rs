//! App state traits for Photon access in generated WebSocket handlers.

use std::sync::Arc;

use photon_runtime::Photon;

/// Implemented by your Axum app state to provide `Arc<Photon>` to
/// macro-generated WebSocket handlers.
pub trait HasPhoton: Clone + Send + Sync + 'static {
    fn photon_arc(&self) -> Arc<Photon>;
}

//! App state traits for Photon access in WebSocket handlers.

use std::sync::Arc;

use photon_runtime::Photon;

use super::hub::WsBroadcastHub;

/// Implemented by Axum app state to provide `Arc<Photon>` to WebSocket handlers.
pub trait HasPhoton: Clone + Send + Sync + 'static {
    /// Return a shared handle to the running Photon runtime.
    fn photon_arc(&self) -> Arc<Photon>;

    /// Optional process-local broadcast hub for [`super::ws::WsFanoutMode::BroadcastHub`].
    ///
    /// Default: `None`. When fanout is `BroadcastHub` and this returns `None`,
    /// the upgrade is rejected (no silent fallback).
    fn ws_hub(&self) -> Option<Arc<WsBroadcastHub>> {
        None
    }

    /// Optional WebSocket `Origin` policy (SEC-002).
    ///
    /// Return `true` only for allowed origins. Hosts MUST override this with an
    /// allowlist for cookie-authenticated production deployments. Demos and
    /// tests that intentionally allow all origins must override this explicitly.
    ///
    /// Returns `false` by default, rejecting upgrades with 403.
    fn allow_ws_origin(&self, _origin: Option<&str>) -> bool {
        false
    }
}

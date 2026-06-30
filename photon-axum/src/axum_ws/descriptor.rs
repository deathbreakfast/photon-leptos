//! WS route descriptor for quark-based auto-discovery.
//!
//! **Audience:** integrators and macro maintainers.
//!
//! Each `#[photon_leptos::synced]` invocation submits a [`WsRouteDescriptor`] via
//! `photon_leptos::inventory::submit!`. [`super::apply_ws_routes`] collects them at runtime.

/// Auth scoping mode for a synced WebSocket endpoint.
#[derive(Debug, Clone, Copy)]
pub enum WsAuthMode {
    /// No authentication — all clients receive all events on the topic.
    None,
    /// User-scoped — events filtered by [`super::PhotonUserExtractor::user_key`].
    User,
}

/// Descriptor for a `#[photon_leptos::synced]` WebSocket route.
///
/// Submitted via `inventory::submit!` by the proc macro and collected at runtime by
/// [`super::apply_ws_routes`].
#[derive(Debug, Clone)]
pub struct WsRouteDescriptor {
    /// Endpoint path (e.g. `"/ws/counter"`).
    pub path: &'static str,
    /// Photon topic name (e.g. `"counter.updated"`).
    pub topic: &'static str,
    /// Auth scoping mode.
    pub auth: WsAuthMode,
}

impl WsRouteDescriptor {
    /// Construct a route descriptor for inventory submission.
    pub const fn new(path: &'static str, topic: &'static str, auth: WsAuthMode) -> Self {
        Self { path, topic, auth }
    }
}

impl quark::Registrable for WsRouteDescriptor {
    fn registry_key(&self) -> &str {
        self.path
    }
}

photon_backend::inventory::collect!(WsRouteDescriptor);

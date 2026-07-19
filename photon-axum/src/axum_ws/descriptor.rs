//! WS route descriptor for quark-based auto-discovery.
//!
//! Each `#[photon_leptos::synced]` invocation submits a [`WsRouteDescriptor`] via
//! `photon_leptos::inventory::submit!`. [`super::apply_ws_routes`] collects them at runtime.

/// Auth scoping mode for a synced WebSocket endpoint.
#[derive(Debug, Clone, Copy)]
pub enum WsAuthMode {
    /// Unauthenticated. Clients may omit `?key=` (broadcast) or supply an
    /// optional client-selected key for partition scoping. Not the same as
    /// authenticated user scoping — hosts must still apply Origin / rate limits.
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
    /// Endpoint path (e.g. `"/ws/notifications"`).
    pub path: &'static str,
    /// Photon topic name (e.g. `"notifications.updated"`).
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

//! WS route descriptor for quark-based auto-discovery.

/// Auth scoping mode for a synced WebSocket endpoint.
#[derive(Debug, Clone, Copy)]
pub enum WsAuthMode {
    /// No authentication — all clients get all events.
    None,
    /// User-scoped — events are filtered by the authenticated user's key.
    User,
}

/// Descriptor for a `#[photon::synced]` WebSocket route, submitted via
/// `inventory::submit!` by the proc macro and collected at runtime by
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

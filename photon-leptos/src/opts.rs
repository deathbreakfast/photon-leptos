//! Options for synced resources.

/// How the resource responds to incoming Photon events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SyncStrategy {
    /// Re-call the server function to fetch fresh data.
    #[default]
    Refetch,

    /// Append the event payload to a list (for `Vec<T>`).
    Append,

    /// Replace resource data with the event payload.
    Replace,
}

impl SyncStrategy {
    /// Parse from string attribute (e.g. `"refetch"`, `"append"`, `"replace"`).
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "refetch" => Some(SyncStrategy::Refetch),
            "append" => Some(SyncStrategy::Append),
            "replace" => Some(SyncStrategy::Replace),
            _ => None,
        }
    }
}

/// Configuration for a synced resource.
#[derive(Debug, Clone)]
pub struct SyncedResourceOpts {
    /// Photon topic name (e.g. `"user.notifications"`).
    pub topic: String,

    /// WebSocket endpoint path (e.g. `"/ws/notifications"`).
    pub ws_path: String,

    /// How to apply incoming events.
    pub strategy: SyncStrategy,

    /// Optional key filter for scoping events (e.g. user_id).
    ///
    /// **`None`:** the client receives every event on [`Self::topic`]. On the Photon **local**
    /// backend this includes keyed publishes; do not use `auth = "none"` for sensitive keyed
    /// topics without an additional gate — prefer `auth = "user"` (or another extractor) so the
    /// server passes `Some(user_key)` here.
    pub key_filter: Option<String>,
}

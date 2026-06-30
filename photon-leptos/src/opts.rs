//! Configuration types for synced Leptos resources.
//!
//! [`SyncStrategy`] and [`SyncedResourceOpts`] control how WebSocket events update
//! UI state when using [`crate::synced_resource`] or macro-generated hooks.

#![warn(missing_docs)]

/// How the resource responds to incoming Photon events.
///
/// | Variant | When to use |
/// |---------|-------------|
/// | [`Refetch`](Self::Refetch) | Server owns query logic (lists, joins, auth-scoped reads) |
/// | [`Replace`](Self::Replace) | WS payload is the full new value (counts, scalars) |
/// | [`Append`](Self::Append) | Feed-style lists — use [`crate::synced_resource_append`] |
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

impl std::str::FromStr for SyncStrategy {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "refetch" => Ok(SyncStrategy::Refetch),
            "append" => Ok(SyncStrategy::Append),
            "replace" => Ok(SyncStrategy::Replace),
            _ => Err(()),
        }
    }
}

/// Configuration for a [`crate::synced_resource`] or macro-generated hook.
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

//! Tests for photon-leptos synced resource and opts.

use photon_leptos::{SyncStrategy, SyncedResourceOpts};

#[test]
fn sync_strategy_from_str() {
    assert_eq!(
        SyncStrategy::from_str("refetch"),
        Some(SyncStrategy::Refetch)
    );
    assert_eq!(SyncStrategy::from_str("append"), Some(SyncStrategy::Append));
    assert_eq!(
        SyncStrategy::from_str("replace"),
        Some(SyncStrategy::Replace)
    );
    assert_eq!(SyncStrategy::from_str("invalid"), None);
}

#[test]
fn synced_resource_opts_builder() {
    let opts = SyncedResourceOpts {
        topic: "user.notifications".to_string(),
        ws_path: "/ws/notifications".to_string(),
        strategy: SyncStrategy::Refetch,
        key_filter: Some("user-123".to_string()),
    };
    assert_eq!(opts.topic, "user.notifications");
    assert_eq!(opts.ws_path, "/ws/notifications");
    assert_eq!(opts.strategy, SyncStrategy::Refetch);
    assert_eq!(opts.key_filter.as_deref(), Some("user-123"));
}

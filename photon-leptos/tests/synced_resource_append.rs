//! Append strategy contract tests (requires `hydrate` for client helpers).

#![cfg(feature = "hydrate")]

use photon_leptos::{synced_resource, SyncStrategy, SyncedResourceOpts};

#[test]
#[should_panic(expected = "synced_resource_append")]
fn synced_resource_append_strategy_panics() {
    let _ = synced_resource(
        || async { 0u64 },
        SyncedResourceOpts {
            topic: "test.append".into(),
            ws_path: "/ws/test-append".into(),
            strategy: SyncStrategy::Append,
            key_filter: None,
        },
    );
}

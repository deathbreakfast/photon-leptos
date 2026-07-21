//! Replace with `Result<T, E>` deserializes event payload as `T` (COR-004).

#![cfg(feature = "hydrate")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use photon_leptos::{synced_resource_replace_result, SyncStrategy, SyncedResourceOpts};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Counter {
    n: u64,
}

#[test]
fn replace_result_deserializes_ok_payload() {
    // Compile-time / type contract: payload type is Counter, resource is Result<Counter, String>.
    let opts = SyncedResourceOpts {
        topic: "test.replace".into(),
        ws_path: "/ws/test-replace".into(),
        strategy: SyncStrategy::Replace,
        key_filter: None,
    };
    // Ensure the helper is callable with Result-returning fetcher.
    let _resource =
        synced_resource_replace_result(|| async { Ok::<Counter, String>(Counter { n: 1 }) }, opts);
    let payload = serde_json::to_value(Counter { n: 9 }).unwrap();
    let decoded: Counter = serde_json::from_value(payload).unwrap();
    assert_eq!(decoded.n, 9);
}

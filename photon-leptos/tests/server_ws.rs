//! Integration tests for photon-leptos server WebSocket handler.
//!
//! These tests require the ssr feature.

#![cfg(feature = "ssr")]

use photon_leptos::server::ws::SyncedWsConfig;

#[test]
fn synced_ws_config_creation() {
    let config = SyncedWsConfig::new("test.topic", Some("user-1".to_string()));
    assert_eq!(config.topic, "test.topic");
    assert_eq!(config.key_filter.as_deref(), Some("user-1"));
}

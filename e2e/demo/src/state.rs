//! Shared demo app state (namespace-keyed counters + scenario flags).

use std::sync::Arc;

use axum::extract::FromRef;
use dashmap::DashMap;
use leptos::prelude::*;
use photon::Photon;
use photon_axum::HasPhoton;

/// Per-namespace E2E scenario toggles (sad-path tests).
#[derive(Clone, Copy, Debug, Default)]
pub struct ScenarioFlags {
    /// Fail the synced read server function.
    pub fail_read: bool,
    /// Fail broadcast publish increments.
    pub fail_publish: bool,
}

/// In-memory counters and scenario state for Playwright workers.
#[derive(Default)]
pub struct CounterStore {
    counters: DashMap<String, u64>,
    partitions: DashMap<String, u64>,
    flags: DashMap<String, ScenarioFlags>,
}

impl CounterStore {
    /// Broadcast counter for `namespace`.
    pub fn get(&self, namespace: &str) -> u64 {
        self.counters.get(namespace).map(|v| *v).unwrap_or(0)
    }

    /// Increment broadcast counter for `namespace`.
    pub fn increment(&self, namespace: &str) -> u64 {
        let mut entry = self.counters.entry(namespace.to_string()).or_insert(0);
        *entry += 1;
        *entry
    }

    /// Partition counter for `(namespace, partition)` (auth/key isolation oracle).
    pub fn get_partition(&self, namespace: &str, partition: &str) -> u64 {
        self.partitions
            .get(&partition_key(namespace, partition))
            .map(|v| *v)
            .unwrap_or(0)
    }

    /// Increment partition counter for `(namespace, partition)`.
    pub fn increment_partition(&self, namespace: &str, partition: &str) -> u64 {
        let mut entry = self
            .partitions
            .entry(partition_key(namespace, partition))
            .or_insert(0);
        *entry += 1;
        *entry
    }

    /// Reset broadcast + partition counters and flags for `namespace`.
    pub fn reset(&self, namespace: &str) {
        self.counters.insert(namespace.to_string(), 0);
        self.flags.remove(namespace);
        let prefix = format!("{namespace}\x1f");
        self.partitions.retain(|k, _| !k.starts_with(&prefix));
    }

    /// Scenario flags for `namespace`.
    pub fn flags(&self, namespace: &str) -> ScenarioFlags {
        self.flags.get(namespace).map(|f| *f).unwrap_or_default()
    }

    /// Update scenario flags for `namespace`.
    pub fn set_scenario(
        &self,
        namespace: &str,
        fail_read: Option<bool>,
        fail_publish: Option<bool>,
    ) {
        let mut entry = self.flags.entry(namespace.to_string()).or_default();
        if let Some(v) = fail_read {
            entry.fail_read = v;
        }
        if let Some(v) = fail_publish {
            entry.fail_publish = v;
        }
    }
}

fn partition_key(namespace: &str, partition: &str) -> String {
    format!("{namespace}\x1f{partition}")
}

/// Axum + Leptos combined state.
#[derive(Clone)]
pub struct AppState {
    /// Leptos configuration.
    pub leptos_options: LeptosOptions,
    /// In-memory counter store.
    pub store: Arc<CounterStore>,
    /// Process-wide Photon handle.
    pub photon: Arc<Photon>,
}

impl HasPhoton for AppState {
    fn photon_arc(&self) -> Arc<Photon> {
        Arc::clone(&self.photon)
    }
}

impl FromRef<AppState> for LeptosOptions {
    fn from_ref(state: &AppState) -> LeptosOptions {
        state.leptos_options.clone()
    }
}

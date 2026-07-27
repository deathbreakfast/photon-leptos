//! Shared app state for the quickstart host.

use std::sync::Arc;

use axum::extract::FromRef;
use dashmap::DashMap;
use leptos::prelude::*;
use photon::Photon;
use photon_axum::HasPhoton;

/// In-memory counters (broadcast + partition).
#[derive(Default)]
pub struct CounterStore {
    counters: DashMap<String, u64>,
    partitions: DashMap<String, u64>,
}

impl CounterStore {
    /// Broadcast counter.
    pub fn get(&self) -> u64 {
        self.counters.get("broadcast").map(|v| *v).unwrap_or(0)
    }

    /// Increment broadcast counter.
    pub fn increment(&self) -> u64 {
        let mut entry = self.counters.entry("broadcast".into()).or_insert(0);
        *entry += 1;
        *entry
    }

    /// Partition counter.
    pub fn get_partition(&self, partition: &str) -> u64 {
        self.partitions.get(partition).map(|v| *v).unwrap_or(0)
    }

    /// Increment partition counter.
    pub fn increment_partition(&self, partition: &str) -> u64 {
        let mut entry = self.partitions.entry(partition.to_string()).or_insert(0);
        *entry += 1;
        *entry
    }
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

    fn allow_ws_origin(&self, _origin: Option<&str>) -> bool {
        // Teaching lab only — see secure-refetch-host for production Origin policy.
        true
    }
}

impl FromRef<AppState> for LeptosOptions {
    fn from_ref(state: &AppState) -> LeptosOptions {
        state.leptos_options.clone()
    }
}

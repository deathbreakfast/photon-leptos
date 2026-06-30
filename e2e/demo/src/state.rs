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
    pub fail_read: bool,
    pub fail_publish: bool,
}

/// In-memory counter and scenario state keyed by Playwright worker namespace.
#[derive(Default)]
pub struct CounterStore {
    counters: DashMap<String, u64>,
    flags: DashMap<String, ScenarioFlags>,
}

impl CounterStore {
    pub fn get(&self, namespace: &str) -> u64 {
        self.counters.get(namespace).map(|v| *v).unwrap_or(0)
    }

    pub fn increment(&self, namespace: &str) -> u64 {
        let mut entry = self.counters.entry(namespace.to_string()).or_insert(0);
        *entry += 1;
        *entry
    }

    pub fn reset(&self, namespace: &str) {
        self.counters.insert(namespace.to_string(), 0);
        self.flags.remove(namespace);
    }

    pub fn flags(&self, namespace: &str) -> ScenarioFlags {
        self.flags.get(namespace).map(|f| *f).unwrap_or_default()
    }

    pub fn set_scenario(
        &self,
        namespace: &str,
        fail_read: Option<bool>,
        fail_publish: Option<bool>,
    ) {
        let mut entry = self
            .flags
            .entry(namespace.to_string())
            .or_default();
        if let Some(v) = fail_read {
            entry.fail_read = v;
        }
        if let Some(v) = fail_publish {
            entry.fail_publish = v;
        }
    }
}

/// Axum + Leptos combined state.
#[derive(Clone)]
pub struct AppState {
    pub leptos_options: LeptosOptions,
    pub store: Arc<CounterStore>,
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

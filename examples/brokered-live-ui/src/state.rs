//! App state.

use std::sync::Arc;

use axum::extract::FromRef;
use dashmap::DashMap;
use leptos::prelude::*;
use photon::Photon;
use photon_axum::HasPhoton;

/// Broadcast counter store.
#[derive(Default)]
pub struct CounterStore {
    value: DashMap<&'static str, u64>,
}

impl CounterStore {
    /// Current value.
    pub fn get(&self) -> u64 {
        self.value.get("v").map(|v| *v).unwrap_or(0)
    }

    /// Increment.
    pub fn increment(&self) -> u64 {
        let mut entry = self.value.entry("v").or_insert(0);
        *entry += 1;
        *entry
    }
}

/// Axum + Leptos state.
#[derive(Clone)]
pub struct AppState {
    /// Leptos options.
    pub leptos_options: LeptosOptions,
    /// Counter store.
    pub store: Arc<CounterStore>,
    /// Photon (NATS-backed).
    pub photon: Arc<Photon>,
}

impl HasPhoton for AppState {
    fn photon_arc(&self) -> Arc<Photon> {
        Arc::clone(&self.photon)
    }

    fn allow_ws_origin(&self, _origin: Option<&str>) -> bool {
        true
    }
}

impl FromRef<AppState> for LeptosOptions {
    fn from_ref(state: &AppState) -> LeptosOptions {
        state.leptos_options.clone()
    }
}

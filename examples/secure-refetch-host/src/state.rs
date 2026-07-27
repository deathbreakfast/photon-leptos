//! App state with Origin allowlist.

use std::sync::Arc;

use axum::extract::FromRef;
use dashmap::DashMap;
use leptos::prelude::*;
use photon::Photon;
use photon_axum::HasPhoton;

/// Env var for comma-separated allowed Origins.
pub const ALLOWED_ORIGINS_ENV: &str = "PHOTON_LEPTOS_ALLOWED_ORIGINS";

/// Per-user counters.
#[derive(Default)]
pub struct CounterStore {
    partitions: DashMap<String, u64>,
}

impl CounterStore {
    /// Partition counter.
    pub fn get(&self, user: &str) -> u64 {
        self.partitions.get(user).map(|v| *v).unwrap_or(0)
    }

    /// Increment partition.
    pub fn increment(&self, user: &str) -> u64 {
        let mut entry = self.partitions.entry(user.to_string()).or_insert(0);
        *entry += 1;
        *entry
    }
}

/// Axum + Leptos state.
#[derive(Clone)]
pub struct AppState {
    /// Leptos configuration.
    pub leptos_options: LeptosOptions,
    /// Counters.
    pub store: Arc<CounterStore>,
    /// Photon handle.
    pub photon: Arc<Photon>,
    /// Allowed WebSocket Origins.
    pub allowed_origins: Arc<[String]>,
}

impl AppState {
    /// Parse allowlist from env or use loopback defaults for port 3021.
    pub fn parse_allowed_origins() -> Arc<[String]> {
        if let Ok(raw) = std::env::var(ALLOWED_ORIGINS_ENV) {
            let list: Vec<String> = raw
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect();
            if !list.is_empty() {
                return Arc::from(list);
            }
        }
        Arc::from(vec![
            "http://127.0.0.1:3021".to_string(),
            "http://localhost:3021".to_string(),
        ])
    }
}

impl HasPhoton for AppState {
    fn photon_arc(&self) -> Arc<Photon> {
        Arc::clone(&self.photon)
    }

    fn allow_ws_origin(&self, origin: Option<&str>) -> bool {
        let Some(origin) = origin else {
            return false;
        };
        self.allowed_origins.iter().any(|allowed| allowed == origin)
    }
}

impl FromRef<AppState> for LeptosOptions {
    fn from_ref(state: &AppState) -> LeptosOptions {
        state.leptos_options.clone()
    }
}

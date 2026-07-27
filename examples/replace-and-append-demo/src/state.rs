//! Shared store for replace snapshot + append log.

use std::sync::{Arc, Mutex};

use axum::extract::FromRef;
use leptos::prelude::*;
use photon::Photon;
use photon_axum::HasPhoton;
use serde::{Deserialize, Serialize};

/// Snapshot type — must match replace topic payload fields.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CounterSnapshot {
    /// Current value.
    pub value: u64,
}

/// Append list item — must match append topic payload.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogLine {
    /// Line text.
    pub text: String,
}

/// In-memory demo state.
#[derive(Default)]
pub struct DemoStore {
    snapshot: Mutex<CounterSnapshot>,
    lines: Mutex<Vec<LogLine>>,
}

impl DemoStore {
    /// Current replace snapshot.
    pub fn snapshot(&self) -> CounterSnapshot {
        self.snapshot
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// Bump replace snapshot.
    pub fn bump(&self) -> CounterSnapshot {
        let mut guard = self
            .snapshot
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.value = guard.value.saturating_add(1);
        guard.clone()
    }

    /// Current append list.
    pub fn lines(&self) -> Vec<LogLine> {
        self.lines
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// Append a line.
    pub fn append(&self, text: String) -> LogLine {
        let line = LogLine { text };
        self.lines
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(line.clone());
        line
    }
}

/// Axum + Leptos state.
#[derive(Clone)]
pub struct AppState {
    /// Leptos options.
    pub leptos_options: LeptosOptions,
    /// Demo store.
    pub store: Arc<DemoStore>,
    /// Photon.
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

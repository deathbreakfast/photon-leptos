//! Replace + Append synced functions.

#![allow(non_snake_case)]

use leptos::prelude::*;
use photon_leptos::synced;
use serde::{Deserialize, Serialize};

#[cfg(feature = "ssr")]
use crate::state::AppState;

/// Replace payload / resource Ok type.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CounterSnapshot {
    /// Current value.
    pub value: u64,
}

/// Append item type.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogLine {
    /// Line text.
    pub text: String,
}

#[cfg(feature = "ssr")]
mod topics {
    use photon::topic;

    #[topic(name = "demo.replace.updated")]
    pub struct ReplaceUpdated {
        pub value: u64,
    }

    #[topic(name = "demo.append.line")]
    pub struct AppendLine {
        pub text: String,
    }
}

/// Replace strategy — event payload is `CounterSnapshot` (`Ok` of Result).
#[server]
#[synced(
    topic = "demo.replace.updated",
    ws = "/ws/replace",
    strategy = "replace",
    auth = "none"
)]
pub async fn replace_counter_get() -> Result<CounterSnapshot, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let state =
            use_context::<AppState>().ok_or_else(|| ServerFnError::new("missing AppState"))?;
        let snap = state.store.snapshot();
        Ok(CounterSnapshot { value: snap.value })
    }
    #[cfg(not(feature = "ssr"))]
    {
        Err(ServerFnError::ServerError("ssr required".into()))
    }
}

/// Append strategy — event payload is one `LogLine`.
#[server]
#[synced(
    topic = "demo.append.line",
    ws = "/ws/append",
    strategy = "append",
    auth = "none"
)]
pub async fn append_log_get() -> Result<Vec<LogLine>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let state =
            use_context::<AppState>().ok_or_else(|| ServerFnError::new("missing AppState"))?;
        Ok(state
            .store
            .lines()
            .into_iter()
            .map(|l| LogLine { text: l.text })
            .collect())
    }
    #[cfg(not(feature = "ssr"))]
    {
        Err(ServerFnError::ServerError("ssr required".into()))
    }
}

#[server]
pub async fn bump_replace() -> Result<(), ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use topics::ReplaceUpdated;
        let state =
            use_context::<AppState>().ok_or_else(|| ServerFnError::new("missing AppState"))?;
        let snap = state.store.bump();
        ReplaceUpdated { value: snap.value }
            .publish()
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        Ok(())
    }
    #[cfg(not(feature = "ssr"))]
    {
        Err(ServerFnError::ServerError("ssr required".into()))
    }
}

#[server]
pub async fn append_line(text: String) -> Result<(), ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use topics::AppendLine;
        let state =
            use_context::<AppState>().ok_or_else(|| ServerFnError::new("missing AppState"))?;
        let line = state.store.append(text);
        AppendLine { text: line.text }
            .publish()
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        Ok(())
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = text;
        Err(ServerFnError::ServerError("ssr required".into()))
    }
}

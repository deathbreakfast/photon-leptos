//! Synced broadcast counter over brokered Photon.

#![allow(non_snake_case)]

use leptos::prelude::*;
use photon_leptos::synced;

#[cfg(feature = "ssr")]
use crate::state::AppState;

#[cfg(feature = "ssr")]
mod topics {
    use photon::topic;

    #[topic(name = "brokered.ui.counter.updated")]
    pub struct BrokeredCounterUpdated;
}

#[server]
#[synced(
    topic = "brokered.ui.counter.updated",
    ws = "/ws/brokered-counter",
    auth = "none"
)]
pub async fn brokered_counter_get() -> Result<u64, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let state =
            use_context::<AppState>().ok_or_else(|| ServerFnError::new("missing AppState"))?;
        Ok(state.store.get())
    }
    #[cfg(not(feature = "ssr"))]
    {
        Err(ServerFnError::ServerError("ssr required".into()))
    }
}

#[server]
pub async fn brokered_increment() -> Result<(), ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use topics::BrokeredCounterUpdated;
        let state =
            use_context::<AppState>().ok_or_else(|| ServerFnError::new("missing AppState"))?;
        state.store.increment();
        BrokeredCounterUpdated {}
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

//! Synced auth-scoped counter + session helpers.

#![allow(non_snake_case)]

use leptos::prelude::*;
use photon_leptos::synced;

#[cfg(feature = "ssr")]
use crate::state::AppState;

/// User id provided during SSR / after login.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionUser(pub String);

#[cfg(feature = "ssr")]
mod topics {
    use photon::topic;

    #[topic(name = "secure.counter.updated", keyed_by = "user")]
    pub struct SecureCounterUpdated {
        pub user: String,
    }
}

#[cfg(feature = "ssr")]
async fn user_from_request() -> Result<String, ServerFnError> {
    if let Some(SessionUser(u)) = use_context::<SessionUser>() {
        return Ok(u);
    }
    use crate::auth::{cookie_value, SESSION_COOKIE};
    use axum::http::HeaderMap;

    let headers: HeaderMap = leptos_axum::extract()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    cookie_value(&headers, SESSION_COOKIE)
        .ok_or_else(|| ServerFnError::new("not signed in"))
}

#[server]
#[synced(
    topic = "secure.counter.updated",
    ws = "/ws/secure-counter",
    auth = "user"
)]
pub async fn secure_counter_get() -> Result<u64, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let user = user_from_request().await?;
        let state =
            use_context::<AppState>().ok_or_else(|| ServerFnError::new("missing AppState"))?;
        Ok(state.store.get(&user))
    }
    #[cfg(not(feature = "ssr"))]
    {
        Err(ServerFnError::ServerError("ssr required".into()))
    }
}

#[server]
pub async fn secure_increment() -> Result<(), ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use topics::SecureCounterUpdated;
        let user = user_from_request().await?;
        let state =
            use_context::<AppState>().ok_or_else(|| ServerFnError::new("missing AppState"))?;
        state.store.increment(&user);
        SecureCounterUpdated { user }
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

/// Returns the Set-Cookie header value for a demo session (path=/).
#[server]
pub async fn sign_in(user: String) -> Result<String, ServerFnError> {
    let user = user.trim().to_string();
    if user.is_empty() || user.len() > 64 {
        return Err(ServerFnError::new("invalid user id"));
    }
    // Demo cookie — production: Secure; HttpOnly; SameSite=Lax|Strict via response headers.
    Ok(format!(
        "demo_session={user}; Path=/; SameSite=Lax; Max-Age=86400"
    ))
}

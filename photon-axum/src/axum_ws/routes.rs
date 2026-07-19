//! Automatic WebSocket route registration via quark auto-discovery.
//!
//! [`apply_ws_routes`] scans inventory for [`WsRouteDescriptor`] entries submitted by
//! `#[photon_leptos::synced]` and mounts Axum GET handlers. User-scoped routes use the
//! generic `Auth` type parameter — pass your host extractor at [`crate::ws_router`].

use axum::extract::ws::WebSocketUpgrade;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, Uri};
use axum::response::{IntoResponse, Response};

use super::auth::PhotonUserExtractor;
use super::descriptor::{WsAuthMode, WsRouteDescriptor};
use super::key_resolve::{resolve_subscribe_key, KeyResolveError};
use super::state::HasPhoton;
use super::ws::{synced_ws_handler, SyncedWsConfig, WsFanoutMode};
use super::ws_query::client_key_from_uri;

/// Register all `#[photon_leptos::synced]` WebSocket routes on the given router.
pub fn apply_ws_routes<S, Auth>(router: axum::Router<S>) -> axum::Router<S>
where
    S: HasPhoton + Clone + Send + Sync + 'static,
    Auth: PhotonUserExtractor + axum::extract::FromRequestParts<S> + Send + 'static,
    <Auth as axum::extract::FromRequestParts<S>>::Rejection: IntoResponse + Send,
{
    let registry = quark::Registry::<WsRouteDescriptor>::auto_discover();

    let mut r = router;
    for desc in registry.iter() {
        match desc.auth {
            WsAuthMode::None => {
                r = mount_none_route(r, desc.path, desc.topic);
            }
            WsAuthMode::User => {
                r = mount_user_route::<S, Auth>(r, desc.path, desc.topic);
            }
        }
    }

    r
}

fn origin_from_headers(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(axum::http::header::ORIGIN)
        .and_then(|v| v.to_str().ok())
}

fn reject_origin() -> Response {
    (StatusCode::FORBIDDEN, "origin not allowed").into_response()
}

fn mount_none_route<S>(
    router: axum::Router<S>,
    path: &'static str,
    topic: &'static str,
) -> axum::Router<S>
where
    S: HasPhoton + Clone + Send + Sync + 'static,
{
    let topic = topic.to_string();
    router.route(
        path,
        axum::routing::get(
            move |ws: WebSocketUpgrade, State(state): State<S>, uri: Uri, headers: HeaderMap| {
                let topic = topic.clone();
                async move {
                    if !state.allow_ws_origin(origin_from_headers(&headers)) {
                        return reject_origin();
                    }
                    let client_key = client_key_from_uri(&uri);
                    match resolve_subscribe_key(WsAuthMode::None, None, client_key.as_deref()) {
                        Ok(key_filter) => respond_upgrade(ws, state, topic, key_filter).await,
                        Err(err) => key_resolve_response(err),
                    }
                }
            },
        ),
    )
}

fn mount_user_route<S, Auth>(
    router: axum::Router<S>,
    path: &'static str,
    topic: &'static str,
) -> axum::Router<S>
where
    S: HasPhoton + Clone + Send + Sync + 'static,
    Auth: PhotonUserExtractor + axum::extract::FromRequestParts<S> + Send + 'static,
    <Auth as axum::extract::FromRequestParts<S>>::Rejection: IntoResponse + Send,
{
    let topic = topic.to_string();
    router.route(
        path,
        axum::routing::get(
            move |ws: WebSocketUpgrade,
                  auth: Auth,
                  State(state): State<S>,
                  uri: Uri,
                  headers: HeaderMap| {
                let topic = topic.clone();
                async move {
                    if !state.allow_ws_origin(origin_from_headers(&headers)) {
                        return reject_origin();
                    }
                    let client_key = client_key_from_uri(&uri);
                    let user_key = auth.user_key();
                    match resolve_subscribe_key(
                        WsAuthMode::User,
                        user_key.as_deref(),
                        client_key.as_deref(),
                    ) {
                        Ok(key_filter) => respond_upgrade(ws, state, topic, key_filter).await,
                        Err(err) => key_resolve_response(err),
                    }
                }
            },
        ),
    )
}

async fn respond_upgrade<S>(
    ws: WebSocketUpgrade,
    state: S,
    topic: String,
    key_filter: Option<String>,
) -> Response
where
    S: HasPhoton,
{
    let fanout = match WsFanoutMode::from_env() {
        Ok(mode) => mode,
        Err(err) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                err.to_string(),
            )
                .into_response();
        }
    };
    let photon = HasPhoton::photon_arc(&state);
    let hub = HasPhoton::ws_hub(&state);
    if matches!(fanout, WsFanoutMode::BroadcastHub) && hub.is_none() {
        return (
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            super::ws::FanoutConfigError::HubRequiredButMissing.to_string(),
        )
            .into_response();
    }
    let config = SyncedWsConfig {
        topic,
        key_filter,
        fanout,
    };
    synced_ws_handler(ws, photon, hub, config).await
}

fn key_resolve_response(err: KeyResolveError) -> Response {
    let status = match &err {
        KeyResolveError::MissingUser => StatusCode::UNAUTHORIZED,
        KeyResolveError::KeyMismatch { .. } => StatusCode::FORBIDDEN,
    };
    if let KeyResolveError::KeyMismatch { .. } = &err {
        // Do not log raw key values (SEC-001).
        photon_backend::instrumentation::log_ops(
            "axum_ws_auth",
            "key_mismatch",
            "client key does not match authenticated user",
            "",
            "",
            "",
        );
    }
    (status, err.client_message().to_string()).into_response()
}

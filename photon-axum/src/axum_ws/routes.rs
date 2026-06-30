//! Automatic WebSocket route registration via quark auto-discovery.
//!
//! [`apply_ws_routes`] scans inventory for [`WsRouteDescriptor`] entries submitted by
//! `#[photon_leptos::synced]` and mounts Axum GET handlers. User-scoped routes use the
//! generic `Auth` type parameter — pass your host extractor at [`crate::ws_router`].

use axum::extract::ws::WebSocketUpgrade;
use axum::extract::State;

use super::auth::PhotonUserExtractor;
use super::descriptor::{WsAuthMode, WsRouteDescriptor};
use super::state::HasPhoton;
use super::ws::{synced_ws_handler, SyncedWsConfig};

/// Register all `#[photon_leptos::synced]` WebSocket routes on the given router.
pub fn apply_ws_routes<S, Auth>(router: axum::Router<S>) -> axum::Router<S>
where
    S: HasPhoton + Clone + Send + Sync + 'static,
    Auth: PhotonUserExtractor + axum::extract::FromRequestParts<S> + Send + 'static,
    <Auth as axum::extract::FromRequestParts<S>>::Rejection: axum::response::IntoResponse + Send,
{
    let registry = quark::Registry::<WsRouteDescriptor>::auto_discover();

    let mut r = router;
    for desc in registry.iter() {
        match desc.auth {
            WsAuthMode::None => {
                let topic = desc.topic.to_string();
                r = r.route(
                    desc.path,
                    axum::routing::get(move |ws: WebSocketUpgrade, State(state): State<S>| {
                        let topic = topic.clone();
                        async move {
                            let config = SyncedWsConfig {
                                topic,
                                key_filter: None,
                                subscription_name: None,
                            };
                            let photon = HasPhoton::photon_arc(&state);
                            synced_ws_handler(ws, photon, config).await
                        }
                    }),
                );
            }
            WsAuthMode::User => {
                let topic = desc.topic.to_string();
                r = r.route(
                    desc.path,
                    axum::routing::get(
                        move |ws: WebSocketUpgrade, auth: Auth, State(state): State<S>| {
                            let topic = topic.clone();
                            async move {
                                let key_filter = auth.user_key();
                                let config = SyncedWsConfig {
                                    topic,
                                    key_filter,
                                    subscription_name: None,
                                };
                                let photon = HasPhoton::photon_arc(&state);
                                synced_ws_handler(ws, photon, config).await
                            }
                        },
                    ),
                );
            }
        }
    }

    r
}

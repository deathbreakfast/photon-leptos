//! Implementation of the `#[photon_leptos::synced]` proc macro.
//!
//! Generates:
//! - A `subscribe_<fn_name>()` subscription helper (all builds)
//! - A `#[cfg(feature = "hydrate")]` client hook `use_<fn_name>()`
//! - A `#[cfg(feature = "ssr")]` module `__photon_ws_<fn_name>` containing
//!   the WebSocket handler and path constant
//! - A `#[cfg(feature = "ssr")]` `inventory::submit!` for WS route auto-discovery

use darling::FromMeta;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, ReturnType};

#[derive(Debug, FromMeta)]
struct SyncedAttrs {
    #[darling(rename = "topic")]
    topic: syn::LitStr,

    /// WebSocket endpoint path. If omitted, derived from the function name:
    /// `get_unread_count` → `/ws/get-unread-count`.
    #[darling(default)]
    ws: Option<syn::LitStr>,

    #[darling(default)]
    strategy: Option<syn::LitStr>,

    #[darling(default)]
    key: Option<syn::LitStr>,

    /// Auth scoping mode for the generated server handler.
    /// - `"none"` (default): no key filter — all clients get all events
    /// - `"user"`: extract user key via host `PhotonUserExtractor` at `ws_router` wiring
    #[darling(default)]
    auth: Option<syn::LitStr>,
}

fn fn_name_to_ws_path(name: &str) -> String {
    format!("/ws/{}", name.replace('_', "-"))
}

/// Expands `#[photon_leptos::synced(...)]` on a server function to generate a
/// client hook and a server-side WebSocket handler module.
pub fn synced_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attrs = match darling::ast::NestedMeta::parse_meta_list(attr.into()) {
        Ok(v) => v,
        Err(e) => return e.to_compile_error().into(),
    };

    let synced_attrs: SyncedAttrs = match SyncedAttrs::from_list(&attrs) {
        Ok(v) => v,
        Err(e) => return e.write_errors().into(),
    };

    let input_fn = parse_macro_input!(item as syn::ItemFn);

    if input_fn.sig.asyncness.is_none() {
        return syn::Error::new_spanned(
            &input_fn.sig,
            "#[photon_leptos::synced] can only be used on async functions",
        )
        .to_compile_error()
        .into();
    }

    let fn_name = &input_fn.sig.ident;
    let fn_vis = &input_fn.vis;

    let subscribe_name = format_ident!("subscribe_{}", fn_name);
    let hook_name = format_ident!("use_{}", fn_name);
    let ws_mod_name = format_ident!("__photon_ws_{}", fn_name);

    let topic_str = synced_attrs.topic.value();

    let ws_path_str = synced_attrs
        .ws
        .as_ref()
        .map(|lit| lit.value())
        .unwrap_or_else(|| fn_name_to_ws_path(&fn_name.to_string()));

    let strategy_str = synced_attrs
        .strategy
        .as_ref()
        .map(|s| s.value())
        .unwrap_or_else(|| "refetch".to_string());

    let strategy = match strategy_str.as_str() {
        "refetch" => quote! { photon_leptos::SyncStrategy::Refetch },
        "append" => quote! { photon_leptos::SyncStrategy::Append },
        "replace" => quote! { photon_leptos::SyncStrategy::Replace },
        _ => {
            return syn::Error::new_spanned(
                synced_attrs
                    .strategy
                    .as_ref()
                    .unwrap_or(&synced_attrs.topic),
                "invalid strategy: expected refetch, append, or replace",
            )
            .to_compile_error()
            .into();
        }
    };

    let auth_str = synced_attrs
        .auth
        .as_ref()
        .map(|s| s.value())
        .unwrap_or_else(|| "none".to_string());

    let key_filter = match &synced_attrs.key {
        Some(k) => quote! { Some(#k.to_string()) },
        None => quote! { None },
    };

    let return_ty = match &input_fn.sig.output {
        ReturnType::Type(_, ty) => ty.clone(),
        _ => {
            return syn::Error::new_spanned(
                &input_fn.sig,
                "#[photon_leptos::synced] function must have an explicit return type",
            )
            .to_compile_error()
            .into();
        }
    };

    let (handler_params, handler_auth_bounds, key_filter_expr) = match auth_str.as_str() {
        "none" => (
            quote! {
                ws: axum::extract::ws::WebSocketUpgrade,
                axum::extract::State(state): axum::extract::State<S>,
            },
            quote! {},
            quote! { None },
        ),
        "user" => (
            quote! {
                ws: axum::extract::ws::WebSocketUpgrade,
                auth: Auth,
                axum::extract::State(state): axum::extract::State<S>,
            },
            quote! {
                Auth: photon_axum::PhotonUserExtractor
                    + axum::extract::FromRequestParts<S>
                    + Send
                    + 'static,
            },
            quote! { auth.user_key() },
        ),
        other => {
            return syn::Error::new_spanned(
                synced_attrs.auth.as_ref().unwrap(),
                format!(
                    "invalid auth mode \"{other}\": expected \"none\" or \"user\""
                ),
            )
            .to_compile_error()
            .into();
        }
    };

    let auth_mode_variant = match auth_str.as_str() {
        "none" => quote! { photon_axum::WsAuthMode::None },
        "user" => quote! { photon_axum::WsAuthMode::User },
        _ => unreachable!(),
    };

    let ws_handler_mod = match auth_str.as_str() {
        "none" => quote! {
            pub async fn handler<S>(
                #handler_params
            ) -> impl axum::response::IntoResponse
            where
                S: photon_axum::HasPhoton + Clone + Send + Sync + 'static,
            {
                let key_filter: Option<String> = #key_filter_expr;

                let config = photon_axum::SyncedWsConfig {
                    topic: #topic_str.to_string(),
                    key_filter,
                    subscription_name: None,
                };

                let photon = photon_axum::HasPhoton::photon_arc(&state);
                photon_axum::synced_ws_handler(ws, photon, config).await
            }
        },
        "user" => quote! {
            pub async fn handler<S, Auth>(
                #handler_params
            ) -> impl axum::response::IntoResponse
            where
                S: photon_axum::HasPhoton + Clone + Send + Sync + 'static,
                #handler_auth_bounds
            {
                let key_filter: Option<String> = #key_filter_expr;

                let config = photon_axum::SyncedWsConfig {
                    topic: #topic_str.to_string(),
                    key_filter,
                    subscription_name: None,
                };

                let photon = photon_axum::HasPhoton::photon_arc(&state);
                photon_axum::synced_ws_handler(ws, photon, config).await
            }
        },
        _ => unreachable!(),
    };

    let expanded = quote! {
        #input_fn

        /// Typed subscription helper generated by `#[photon_leptos::synced]`.
        ///
        /// Subscribes to the WebSocket endpoint and calls `on_event` for
        /// each incoming event. Returns a trigger signal that bumps on
        /// every event — use as a `Resource` source for automatic refetch.
        ///
        /// Works in all builds: on SSR the WebSocket call compiles out and
        /// the trigger stays at 0 (initial server fetch only).
        #fn_vis fn #subscribe_name(
            on_event: impl Fn() + Send + Sync + 'static,
        ) -> leptos::prelude::RwSignal<u64> {
            let trigger = leptos::prelude::RwSignal::new(0u64);
            #[cfg(feature = "hydrate")]
            photon_leptos::subscribe_ws(#ws_path_str, move |_payload| {
                trigger.update(|n| *n += 1);
                on_event();
            });
            trigger
        }

        /// Client-side hook generated by `#[photon_leptos::synced]`.
        /// Creates a `Resource` that automatically refetches when a Photon
        /// event arrives on the WebSocket.
        #[cfg(feature = "hydrate")]
        #fn_vis fn #hook_name() -> leptos::prelude::Resource<#return_ty> {
            photon_leptos::synced_resource(
                #fn_name,
                photon_leptos::SyncedResourceOpts {
                    topic: #topic_str.to_string(),
                    ws_path: #ws_path_str.to_string(),
                    strategy: #strategy,
                    key_filter: #key_filter,
                },
            )
        }

        /// Server-side WebSocket handler module generated by `#[photon_leptos::synced]`.
        #[cfg(feature = "ssr")]
        #fn_vis mod #ws_mod_name {
            /// WebSocket endpoint path for this synced resource.
            pub const PATH: &str = #ws_path_str;

            #ws_handler_mod
        }

        #[cfg(feature = "ssr")]
        photon_leptos::inventory::submit! {
            photon_axum::WsRouteDescriptor::new(
                #ws_path_str,
                #topic_str,
                #auth_mode_variant,
            )
        }
    };

    expanded.into()
}

#[cfg(test)]
mod tests {
    use super::fn_name_to_ws_path;

    #[test]
    fn ws_path_from_fn_name() {
        assert_eq!(fn_name_to_ws_path("counter_get"), "/ws/counter-get");
        assert_eq!(fn_name_to_ws_path("get_unread_count"), "/ws/get-unread-count");
    }
}

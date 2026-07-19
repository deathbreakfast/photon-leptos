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

    /// Static Photon subscribe key sent as `?key=` from generated client helpers.
    #[darling(default)]
    key: Option<syn::LitStr>,

    /// Auth scoping mode for the generated server handler.
    /// - `"none"` (default): broadcast, or keyed when client sends `?key=`
    /// - `"user"`: key from host `PhotonUserExtractor` (optional matching `?key=`)
    #[darling(default)]
    auth: Option<syn::LitStr>,
}

fn fn_name_to_ws_path(name: &str) -> String {
    format!("/ws/{}", name.replace('_', "-"))
}

/// `Result<Vec<_>, _>` (or `Result<Vec<_>, _>` behind a path segment like `ServerFnError`).
fn return_type_is_result_of_vec(ty: &syn::Type) -> bool {
    result_ok_type(ty).is_some_and(|ok| {
        let syn::Type::Path(ok_path) = ok else {
            return false;
        };
        ok_path
            .path
            .segments
            .last()
            .is_some_and(|s| s.ident == "Vec")
    })
}

/// If `ty` is `Result<Ok, Err>`, return `Ok`.
fn result_ok_type(ty: &syn::Type) -> Option<&syn::Type> {
    let syn::Type::Path(type_path) = ty else {
        return None;
    };
    let seg = type_path.path.segments.last()?;
    if seg.ident != "Result" {
        return None;
    }
    let syn::PathArguments::AngleBracketed(args) = &seg.arguments else {
        return None;
    };
    match args.args.first()? {
        syn::GenericArgument::Type(ok_ty) => Some(ok_ty),
        _ => None,
    }
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

    let is_append = strategy_str == "append";
    let is_replace = strategy_str == "replace";

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

    let replace_uses_result_ok = is_replace && result_ok_type(&return_ty).is_some();

    if is_append && !return_type_is_result_of_vec(&return_ty) {
        return syn::Error::new_spanned(
            synced_attrs
                .strategy
                .as_ref()
                .unwrap_or(&synced_attrs.topic),
            "strategy = \"append\" requires the function to return Result<Vec<_>, _>",
        )
        .to_compile_error()
        .into();
    }

    let opts_fields = quote! {
        topic: #topic_str.to_string(),
        ws_path: #ws_path_str.to_string(),
        strategy: #strategy,
        key_filter: #key_filter,
    };

    let hydrate_hook = if is_append {
        quote! {
            /// Client-side hook generated by `#[photon_leptos::synced]`.
            /// Creates a `Resource` that appends WS payloads to the list.
            #[cfg(feature = "hydrate")]
            #fn_vis fn #hook_name() -> leptos::prelude::Resource<Option<#return_ty>> {
                photon_leptos::synced_resource_append(
                    #fn_name,
                    photon_leptos::SyncedResourceOpts {
                        #opts_fields
                    },
                )
            }
        }
    } else if replace_uses_result_ok {
        quote! {
            /// Client-side hook generated by `#[photon_leptos::synced]`.
            /// Replace strategy: event payload is the `Ok` value of `Result<T, E>`.
            #[cfg(feature = "hydrate")]
            #fn_vis fn #hook_name() -> leptos::prelude::Resource<#return_ty> {
                photon_leptos::synced_resource_replace_result(
                    #fn_name,
                    photon_leptos::SyncedResourceOpts {
                        #opts_fields
                    },
                )
            }
        }
    } else {
        quote! {
            /// Client-side hook generated by `#[photon_leptos::synced]`.
            /// Creates a `Resource` that automatically refetches when a Photon
            /// event arrives on the WebSocket.
            #[cfg(feature = "hydrate")]
            #fn_vis fn #hook_name() -> leptos::prelude::Resource<#return_ty> {
                photon_leptos::synced_resource(
                    #fn_name,
                    photon_leptos::SyncedResourceOpts {
                        #opts_fields
                    },
                )
            }
        }
    };

    let auth_mode_variant = match auth_str.as_str() {
        "none" => quote! { photon_leptos::server::WsAuthMode::None },
        "user" => quote! { photon_leptos::server::WsAuthMode::User },
        other => {
            return syn::Error::new_spanned(
                synced_attrs.auth.as_ref().unwrap(),
                format!("invalid auth mode \"{other}\": expected \"none\" or \"user\""),
            )
            .to_compile_error()
            .into();
        }
    };

    let ws_handler_mod = match auth_str.as_str() {
        "none" => quote! {
            /// Manual registration helper (inventory `ws_router` is preferred).
            pub async fn handler<S>(
                ws: axum::extract::ws::WebSocketUpgrade,
                axum::extract::State(state): axum::extract::State<S>,
                uri: axum::http::Uri,
            ) -> axum::response::Response
            where
                S: photon_leptos::server::HasPhoton + Clone + Send + Sync + 'static,
            {
                use axum::response::IntoResponse;
                let client_key = photon_leptos::server::client_key_from_uri(&uri);
                let key_filter = match photon_leptos::server::resolve_subscribe_key(
                    photon_leptos::server::WsAuthMode::None,
                    None,
                    client_key.as_deref(),
                ) {
                    Ok(k) => k,
                    Err(e) => {
                        return (
                            axum::http::StatusCode::BAD_REQUEST,
                            e.client_message(),
                        )
                            .into_response();
                    }
                };
                let config = photon_leptos::server::SyncedWsConfig::new(#topic_str, key_filter);
                let photon = photon_leptos::server::HasPhoton::photon_arc(&state);
                let hub = photon_leptos::server::HasPhoton::ws_hub(&state);
                photon_leptos::server::synced_ws_handler(ws, photon, hub, config).await
            }
        },
        "user" => quote! {
            /// Manual registration helper (inventory `ws_router` is preferred).
            pub async fn handler<S, Auth>(
                ws: axum::extract::ws::WebSocketUpgrade,
                auth: Auth,
                axum::extract::State(state): axum::extract::State<S>,
                uri: axum::http::Uri,
            ) -> axum::response::Response
            where
                S: photon_leptos::server::HasPhoton + Clone + Send + Sync + 'static,
                Auth: photon_leptos::server::PhotonUserExtractor
                    + axum::extract::FromRequestParts<S>
                    + Send
                    + 'static,
            {
                use axum::response::IntoResponse;
                let client_key = photon_leptos::server::client_key_from_uri(&uri);
                let user_key = auth.user_key();
                let key_filter = match photon_leptos::server::resolve_subscribe_key(
                    photon_leptos::server::WsAuthMode::User,
                    user_key.as_deref(),
                    client_key.as_deref(),
                ) {
                    Ok(k) => k,
                    Err(photon_leptos::server::KeyResolveError::MissingUser) => {
                        return (
                            axum::http::StatusCode::UNAUTHORIZED,
                            "auth=user requires an authenticated user key",
                        )
                            .into_response();
                    }
                    Err(e) => {
                        return (
                            axum::http::StatusCode::FORBIDDEN,
                            e.client_message(),
                        )
                            .into_response();
                    }
                };
                let config = photon_leptos::server::SyncedWsConfig::new(#topic_str, key_filter);
                let photon = photon_leptos::server::HasPhoton::photon_arc(&state);
                let hub = photon_leptos::server::HasPhoton::ws_hub(&state);
                photon_leptos::server::synced_ws_handler(ws, photon, hub, config).await
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
            {
                let key_filter: Option<String> = #key_filter;
                let _ws = photon_leptos::subscribe_ws(
                    #ws_path_str,
                    key_filter.as_deref(),
                    move |_payload| {
                        trigger.update(|n| *n += 1);
                        on_event();
                    },
                );
            }
            trigger
        }

        #hydrate_hook

        /// Server-side WebSocket handler module generated by `#[photon_leptos::synced]`.
        #[cfg(feature = "ssr")]
        #fn_vis mod #ws_mod_name {
            /// WebSocket endpoint path for this synced resource.
            pub const PATH: &str = #ws_path_str;

            #ws_handler_mod
        }

        #[cfg(feature = "ssr")]
        photon_leptos::inventory::submit! {
            photon_leptos::server::WsRouteDescriptor::new(
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
    use super::{fn_name_to_ws_path, result_ok_type, return_type_is_result_of_vec};
    use syn::parse_quote;

    #[test]
    fn ws_path_from_fn_name() {
        assert_eq!(fn_name_to_ws_path("counter_get"), "/ws/counter-get");
        assert_eq!(
            fn_name_to_ws_path("get_unread_count"),
            "/ws/get-unread-count"
        );
    }

    #[test]
    fn result_vec_return_type_detection() {
        let ok: syn::Type = parse_quote!(Result<Vec<Item>, ServerFnError>);
        let also_ok: syn::Type = parse_quote!(Result<Vec<String>, String>);
        let not_vec: syn::Type = parse_quote!(Result<u64, ServerFnError>);
        let not_result: syn::Type = parse_quote!(Vec<Item>);
        assert!(return_type_is_result_of_vec(&ok));
        assert!(return_type_is_result_of_vec(&also_ok));
        assert!(!return_type_is_result_of_vec(&not_vec));
        assert!(!return_type_is_result_of_vec(&not_result));
    }

    #[test]
    fn result_ok_type_extraction() {
        let result_counter: syn::Type = parse_quote!(Result<Counter, ServerFnError>);
        let ok = result_ok_type(&result_counter).expect("Result Ok");
        let expected: syn::Type = parse_quote!(Counter);
        assert_eq!(
            quote::ToTokens::to_token_stream(ok).to_string(),
            quote::ToTokens::to_token_stream(&expected).to_string()
        );
        let plain: syn::Type = parse_quote!(Counter);
        assert!(result_ok_type(&plain).is_none());
    }
}

//! Leptos pages for the counter E2E demo.

use leptos::prelude::*;
use leptos_router::hooks::use_query_map;

use crate::counter::{counter_get, counter_get_auth_user};

/// Mirror the `ns` query param into a cookie so client-side server-fn refetches work.
fn sync_namespace_cookie(_namespace: &str) {
    #[cfg(feature = "hydrate")]
    {
        let namespace = _namespace;
        use wasm_bindgen::JsCast;
        if let Some(document) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.dyn_into::<web_sys::HtmlDocument>().ok())
        {
            let _ = document.set_cookie(&format!("e2e_ns={namespace}; path=/"));
        }
    }
}

#[component]
pub fn CounterPage() -> impl IntoView {
    let query = use_query_map();
    let namespace = move || {
        query
            .get()
            .get("ns")
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "default".to_string())
    };
    let use_ws = move || query.get().get("mode").as_deref() != Some("no-ws");

    view! {
        <Show
            when=use_ws
            fallback=move || view! { <NoWsCounterView namespace=namespace /> }
        >
            <SyncedCounterView namespace=namespace />
        </Show>
    }
}

#[component]
pub fn AuthMismatchPage() -> impl IntoView {
    let query = use_query_map();
    let namespace = move || {
        query
            .get()
            .get("ns")
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "default".to_string())
    };

    view! {
        <AuthCounterView namespace=namespace />
    }
}

#[component]
fn SyncedCounterView(namespace: impl Fn() -> String + Send + Sync + Clone + 'static) -> impl IntoView {
    provide_context(namespace());
    sync_namespace_cookie(&namespace());

    let ws_status = RwSignal::new("connected".to_string());
    let trigger = crate::counter::subscribe_counter_get(|| {});
    let counter = Resource::new(move || trigger.get(), move |_| counter_get());

    view! {
        <span data-testid="ws-status">{move || ws_status.get()}</span>
        <CounterDisplay resource=counter />
    }
}

#[component]
fn NoWsCounterView(namespace: impl Fn() -> String + Send + Sync + Clone + 'static) -> impl IntoView {
    provide_context(namespace());
    sync_namespace_cookie(&namespace());

    let counter = Resource::new(|| (), move |_| counter_get());

    view! {
        <span data-testid="ws-status">"disabled"</span>
        <CounterDisplay resource=counter />
    }
}

#[component]
fn AuthCounterView(namespace: impl Fn() -> String + Send + Sync + Clone + 'static) -> impl IntoView {
    provide_context(namespace());
    sync_namespace_cookie(&namespace());

    let trigger = crate::counter::subscribe_counter_get_auth_user(|| {});
    let counter = Resource::new(move || trigger.get(), move |_| counter_get_auth_user());

    view! {
        <CounterDisplay resource=counter />
    }
}

#[component]
fn CounterDisplay(
    resource: Resource<Result<u64, ServerFnError>>,
) -> impl IntoView {
    view! {
        <Suspense fallback=move || view! {
            <span data-testid="counter-loading">"…"</span>
        }>
            {move || match resource.get() {
                Some(Ok(value)) => view! {
                    <span data-testid="counter-value">{value.to_string()}</span>
                }.into_any(),
                Some(Err(err)) => view! {
                    <span data-testid="counter-error">{err.to_string()}</span>
                }.into_any(),
                None => view! {
                    <span data-testid="counter-loading">"…"</span>
                }.into_any(),
            }}
        </Suspense>
    }
}

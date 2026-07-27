//! Brokered counter UI.

use leptos::prelude::*;

use crate::synced::{brokered_counter_get, brokered_increment};

#[component]
pub fn HomePage() -> impl IntoView {
    let trigger = crate::synced::subscribe_brokered_counter_get(|| {});
    let counter = Resource::new(move || trigger.get(), move |_| brokered_counter_get());

    let on_increment = move |_| {
        leptos::task::spawn_local(async {
            let _ = brokered_increment().await;
        });
    };

    view! {
        <h1>"Brokered live UI"</h1>
        <p>
            "Photon storage is NATS JetStream ("
            <code>"PHOTON_NATS_URL"</code>
            "). Browser path is still WS refetch via "
            <code>"#[synced]"</code>
            "."
        </p>
        <div class="row">
            <span>"Counter: "</span>
            <Suspense fallback=move || view! { <span>"…"</span> }>
                {move || match counter.get() {
                    Some(Ok(v)) => view! { <strong>{v.to_string()}</strong> }.into_any(),
                    Some(Err(e)) => view! { <span>{e.to_string()}</span> }.into_any(),
                    None => view! { <span>"…"</span> }.into_any(),
                }}
            </Suspense>
            <button type="button" on:click=on_increment>"Increment"</button>
        </div>
    }
}

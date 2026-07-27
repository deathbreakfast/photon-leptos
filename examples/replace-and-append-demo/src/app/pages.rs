//! Replace + Append UI.

use leptos::prelude::*;

use crate::synced::{append_line, bump_replace, CounterSnapshot, LogLine};

#[component]
pub fn HomePage() -> impl IntoView {
    view! {
        <h1>"Replace & Append"</h1>
        <p>
            "Replace applies "
            <code>"payload_json"</code>
            " as "
            <code>"T"</code>
            ". Append pushes each event as one list item (best-effort; no reconnect replay)."
        </p>
        <ReplaceSection/>
        <AppendSection/>
    }
}

#[component]
fn ReplaceSection() -> impl IntoView {
    #[cfg(feature = "hydrate")]
    let resource = crate::synced::use_replace_counter_get();
    #[cfg(not(feature = "hydrate"))]
    let resource = Resource::new(|| (), |_| crate::synced::replace_counter_get());

    let on_bump = move |_| {
        leptos::task::spawn_local(async {
            let _ = bump_replace().await;
        });
    };

    view! {
        <section>
            <h2>"Replace"</h2>
            <div class="row">
                <span>"Snapshot value: "</span>
                <Suspense fallback=move || view! { <span>"…"</span> }>
                    {move || match resource.get() {
                        Some(Ok(CounterSnapshot { value })) => {
                            view! { <strong>{value.to_string()}</strong> }.into_any()
                        }
                        Some(Err(e)) => view! { <span>{e.to_string()}</span> }.into_any(),
                        None => view! { <span>"…"</span> }.into_any(),
                    }}
                </Suspense>
                <button type="button" on:click=on_bump>"Bump replace"</button>
            </div>
        </section>
    }
}

#[component]
fn AppendSection() -> impl IntoView {
    #[cfg(feature = "hydrate")]
    let resource = crate::synced::use_append_log_get();
    #[cfg(not(feature = "hydrate"))]
    let resource = Resource::new(|| (), async |_| Some(crate::synced::append_log_get().await));

    let draft = RwSignal::new("hello".to_string());

    let on_append = move |_| {
        let text = draft.get();
        leptos::task::spawn_local(async move {
            let _ = append_line(text).await;
        });
    };

    view! {
        <section>
            <h2>"Append"</h2>
            <div class="row">
                <input
                    type="text"
                    prop:value=move || draft.get()
                    on:input=move |ev| draft.set(event_target_value(&ev))
                />
                <button type="button" on:click=on_append>"Append line"</button>
            </div>
            <Suspense fallback=move || view! { <p>"…"</p> }>
                {move || {
                    #[cfg(feature = "hydrate")]
                    {
                        match resource.get() {
                            Some(Some(Ok(lines))) => view! {
                                <ul>
                                    {lines.into_iter().map(|LogLine { text }| view! {
                                        <li>{text}</li>
                                    }).collect_view()}
                                </ul>
                            }.into_any(),
                            Some(Some(Err(e))) => view! { <p>{e.to_string()}</p> }.into_any(),
                            Some(None) | None => view! { <p>"…"</p> }.into_any(),
                        }
                    }
                    #[cfg(not(feature = "hydrate"))]
                    {
                        match resource.get() {
                            Some(Some(Ok(lines))) => view! {
                                <ul>
                                    {lines.into_iter().map(|LogLine { text }| view! {
                                        <li>{text}</li>
                                    }).collect_view()}
                                </ul>
                            }.into_any(),
                            Some(Some(Err(e))) => view! { <p>{e.to_string()}</p> }.into_any(),
                            Some(None) | None => view! { <p>"…"</p> }.into_any(),
                        }
                    }
                }}
            </Suspense>
        </section>
    }
}

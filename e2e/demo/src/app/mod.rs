//! Leptos application shell and routing.

mod pages;

use leptos::hydration::{AutoReload, HydrationScripts};
use leptos::prelude::*;
use leptos_router::components::{Route, Router, Routes};
use leptos_router::StaticSegment;

pub use pages::{AuthKeyPage, AuthOnlyPage, CounterPage, KeyOnlyPage};

/// HTML shell for SSR.
pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <title>"photon-leptos E2E"</title>
                <AutoReload options=options.clone() />
                <HydrationScripts options=options.clone() />
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

/// Root router.
#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <Routes fallback=move || view! { <p>"Not found"</p> }>
                <Route path=StaticSegment("") view=CounterPage/>
                <Route path=StaticSegment("auth-only") view=AuthOnlyPage/>
                <Route path=StaticSegment("key-only") view=KeyOnlyPage/>
                <Route path=StaticSegment("auth-key") view=AuthKeyPage/>
            </Routes>
        </Router>
    }
}

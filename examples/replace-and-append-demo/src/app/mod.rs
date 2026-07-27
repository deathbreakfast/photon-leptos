//! App shell.

mod pages;

use leptos::hydration::{AutoReload, HydrationScripts};
use leptos::prelude::*;
use leptos_router::components::{Route, Router, Routes};
use leptos_router::StaticSegment;

pub use pages::HomePage;

/// HTML shell.
pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <title>"replace-and-append-demo"</title>
                <AutoReload options=options.clone() />
                <HydrationScripts options=options />
                <style>{r#"
                    body { font-family: system-ui, sans-serif; margin: 2rem; max-width: 44rem; }
                    section { margin: 1.5rem 0; padding-bottom: 1rem; border-bottom: 1px solid #ddd; }
                    .row { display: flex; gap: 0.75rem; align-items: center; margin: 0.5rem 0; flex-wrap: wrap; }
                    button, input { padding: 0.4rem 0.8rem; }
                    ul { padding-left: 1.25rem; }
                    code { background: #f4f4f4; padding: 0.1rem 0.3rem; }
                "#}</style>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

/// Root.
#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <Routes fallback=move || view! { <p>"Not found"</p> }>
                <Route path=StaticSegment("") view=HomePage/>
            </Routes>
        </Router>
    }
}

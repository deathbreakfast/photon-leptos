pub mod app;
pub mod counter;
#[cfg(feature = "ssr")]
pub mod photon_boot;
#[cfg(feature = "ssr")]
pub mod state;

pub use app::{App, shell};

#[cfg(feature = "ssr")]
pub use state::{AppState, CounterStore};

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use app::App;
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}

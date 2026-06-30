//! Shared bench server state.

use std::sync::Arc;

use photon::Photon;
use photon_axum::HasPhoton;

#[derive(Clone)]
pub struct BenchState {
    pub photon: Arc<Photon>,
}

impl HasPhoton for BenchState {
    fn photon_arc(&self) -> Arc<Photon> {
        Arc::clone(&self.photon)
    }
}

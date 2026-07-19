//! Shared bench server state.

use std::sync::{Arc, RwLock};

use photon::Photon;
use photon_axum::{HasPhoton, WsBroadcastHub, WsFanoutMode};

#[derive(Clone)]
pub struct BenchState {
    pub photon: Arc<Photon>,
    pub hub: Arc<WsBroadcastHub>,
    pub fanout: Arc<RwLock<WsFanoutMode>>,
}

impl BenchState {
    pub fn new(photon: Arc<Photon>, fanout: WsFanoutMode) -> Self {
        Self {
            photon,
            hub: Arc::new(WsBroadcastHub::new()),
            fanout: Arc::new(RwLock::new(fanout)),
        }
    }

    pub fn fanout_mode(&self) -> WsFanoutMode {
        *self.fanout.read().unwrap_or_else(|e| e.into_inner())
    }

    pub fn set_fanout_mode(&self, mode: WsFanoutMode) {
        *self.fanout.write().unwrap_or_else(|e| e.into_inner()) = mode;
        std::env::set_var("PHOTON_AXUM_WS_FANOUT", mode.as_str());
    }
}

impl HasPhoton for BenchState {
    fn photon_arc(&self) -> Arc<Photon> {
        Arc::clone(&self.photon)
    }

    fn ws_hub(&self) -> Option<Arc<WsBroadcastHub>> {
        Some(Arc::clone(&self.hub))
    }
}

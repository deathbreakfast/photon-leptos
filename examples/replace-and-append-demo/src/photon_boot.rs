//! Mem Photon boot.

use std::sync::Arc;

use anyhow::Result;
use photon::{configure, Photon};

/// Build and configure Photon.
pub fn build_photon() -> Result<Arc<Photon>> {
    let photon = Photon::builder().auto_registry().build()?;
    configure(photon.clone());
    Ok(Arc::new(photon))
}

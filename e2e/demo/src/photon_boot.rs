//! In-process mem Photon backend for the E2E demo.

use std::sync::Arc;

use anyhow::Result;
use photon::{configure, Photon};

/// Build and configure the process-wide [`Photon`] instance.
pub fn build_photon() -> Result<Arc<Photon>> {
    let photon = Photon::builder().auto_registry().build()?;
    configure(photon.clone());
    Ok(Arc::new(photon))
}

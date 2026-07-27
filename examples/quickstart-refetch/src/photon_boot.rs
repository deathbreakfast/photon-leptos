//! In-process mem Photon for the quickstart host.

use std::sync::Arc;

use anyhow::Result;
use photon::{configure, Photon};

/// Build and configure the process-wide [`Photon`] instance.
pub fn build_photon() -> Result<Arc<Photon>> {
    let photon = Photon::builder().auto_registry().build()?;
    configure(photon.clone());
    Ok(Arc::new(photon))
}

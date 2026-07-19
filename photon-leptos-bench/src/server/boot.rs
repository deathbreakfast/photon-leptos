//! In-process mem Photon backend for benchmarks.

use std::sync::Arc;

use anyhow::Result;
use photon::{configure, Photon};
use photon_testkit::BootstrapSession;

/// Build process-wide [`Photon`] with in-process mem storage (fixed bench config).
pub async fn build_photon() -> Result<Arc<Photon>> {
    let mut session = BootstrapSession::new(photon_testkit::MatrixSpec::ci_mem_embedded());
    session.install_async().await?;
    let photon = session.build_photon()?;
    configure(photon.clone());
    Ok(Arc::new(photon))
}

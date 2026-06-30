//! SQLite Continuum + Photon embedded backend for benchmarks.

use std::sync::Arc;

use anyhow::Result;
use photon::{configure, Photon};
use photon_testkit::matrix::Topology;
use photon_testkit::BootstrapSession;

/// Build process-wide [`Photon`] with sqlite storage (fixed bench config).
pub async fn build_photon() -> Result<Arc<Photon>> {
    let mut session = BootstrapSession::new(
        photon_testkit::MatrixSpec::ci_sqlite_embedded().with_topology(Topology::EmbeddedComposite),
    );
    session.install_async().await?;
    let photon = session.build_photon()?;
    configure(photon.clone());
    Ok(Arc::new(photon))
}

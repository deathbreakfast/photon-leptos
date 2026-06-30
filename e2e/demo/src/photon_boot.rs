//! Continuum mem transport + Photon embedded backend for the E2E demo.

use std::sync::Arc;

use anyhow::Result;
use continuum::backends::InMemoryLogBackend;
use continuum::router::LogRouter;
use continuum::types::{LogBackendKind, LogDestination};
use photon::{configure, EmbeddedBackend, Photon, TransportCrypto, TransportStore};

/// Install the in-memory Continuum router (once per process).
pub fn install_mem_router() {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| {
        let destination = LogDestination::new("default", LogBackendKind::Memory);
        let router = LogRouter::with_default(
            &destination,
            Arc::new(InMemoryLogBackend::new()) as Arc<dyn continuum::backend::LogBackend>,
        );
        LogRouter::set_global(router);
        photon::transport::set_transport_destination(destination);
    });
}

/// Build and configure the process-wide [`Photon`] instance.
pub fn build_photon() -> Result<Arc<Photon>> {
    install_mem_router();
    let crypto = TransportCrypto::from_env_or_dev_default();
    let transport = TransportStore::from_global(crypto)?;
    let photon = Photon::builder()
        .transport_store(transport)
        .backend_with_context(EmbeddedBackend::install)
        .auto_registry()
        .build()?;
    configure(photon.clone());
    Ok(Arc::new(photon))
}

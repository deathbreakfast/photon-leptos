//! NATS JetStream Photon boot (skip when `PHOTON_NATS_URL` unset).

use std::sync::Arc;

use anyhow::Result;
use photon::{configure, NatsStoragePort, Photon, ReplayCursor};

/// Result of attempting a NATS-backed Photon boot.
pub enum NatsBootOutcome {
    /// Photon ready.
    Ready(Arc<Photon>),
    /// Env unset — runbook printed; caller should exit Ok.
    Skipped,
}

/// Build Photon on NATS, or skip with a runbook when `PHOTON_NATS_URL` is unset.
pub async fn build_photon_nats() -> Result<NatsBootOutcome> {
    if std::env::var_os("PHOTON_NATS_URL").is_none() {
        tracing::warn!(
            "brokered-live-ui: PHOTON_NATS_URL unset — skipping.\n\
             \n\
             docker run -d --name photon-nats -p 4222:4222 nats:2.10 -js\n\
             export PHOTON_TRANSPORT_KEY=cGhvdG9uLWRldi10cmFuc3BvcnQta2V5LTMyYnl0ZXM=\n\
             export PHOTON_NATS_URL=nats://127.0.0.1:4222\n\
             export PHOTON_NATS_STREAM=photon\n\
             export PHOTON_ALLOW_INSECURE_BROKER=1\n\
             cargo leptos watch --split --project brokered-live-ui\n"
        );
        return Ok(NatsBootOutcome::Skipped);
    }

    let port = Arc::new(
        NatsStoragePort::builder()
            .from_env_defaults()
            .replay_cursor(ReplayCursor::StreamSeq)
            .sync_ack(true)
            .build()
            .await?,
    );
    let photon = Photon::builder()
        .storage_port(port)
        .auto_registry()
        .build()?;
    configure(photon.clone());
    tracing::info!("brokered-live-ui: NATS storage port ready");
    Ok(NatsBootOutcome::Ready(Arc::new(photon)))
}

//! Standalone bench server binary.

use std::net::SocketAddr;

use photon_leptos_bench::server::{build_photon, build_router, BenchState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let addr: SocketAddr = std::env::var("BENCH_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:8080".into())
        .parse()?;
    let photon = build_photon().await?;
    let state = BenchState { photon };
    let app = build_router(state);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    eprintln!("photon-leptos-bench-server listening on http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}

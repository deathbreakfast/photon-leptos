//! Bind-address guards for the bench server.

use std::net::SocketAddr;

/// Environment opt-in to bind the bench server on a non-loopback address.
pub const BENCH_ALLOW_NONLOCAL_ENV: &str = "PHOTON_LEPTOS_BENCH_ALLOW_NONLOCAL";

/// Refuse public binds unless explicitly opted in.
///
/// The bench data plane is intentionally unauthenticated for load generation on loopback.
///
/// # Errors
///
/// Returns an error when the address is not loopback and the env opt-in is unset.
pub fn ensure_bench_bind_allowed(addr: SocketAddr) -> Result<(), String> {
    if addr.ip().is_loopback() {
        return Ok(());
    }
    match std::env::var(BENCH_ALLOW_NONLOCAL_ENV).as_deref() {
        Ok("1" | "true" | "TRUE" | "yes" | "YES") => Ok(()),
        _ => Err(format!(
            "photon-leptos-bench refuses non-loopback bind {addr}. \
             Use 127.0.0.1 or set {BENCH_ALLOW_NONLOCAL_ENV}=1 for lab-only exposure"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn loopback_allowed() {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8080);
        ensure_bench_bind_allowed(addr).expect("loopback");
    }

    #[test]
    fn nonlocal_rejected_without_opt_in() {
        std::env::remove_var(BENCH_ALLOW_NONLOCAL_ENV);
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 8080);
        let err = ensure_bench_bind_allowed(addr).expect_err("public");
        assert!(err.contains(BENCH_ALLOW_NONLOCAL_ENV));
    }
}

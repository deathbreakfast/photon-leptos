//! Bind-address guards for the insecure E2E demo.

use std::net::SocketAddr;

/// Environment opt-in to bind the demo on a non-loopback address.
pub const DEMO_ALLOW_INSECURE_ENV: &str = "PHOTON_LEPTOS_DEMO_ALLOW_INSECURE";

/// Refuse public binds unless explicitly opted in.
///
/// The E2E demo intentionally uses insecure defaults (allow-all Origin, query-param identity).
/// It may bind loopback freely. Non-loopback requires [`DEMO_ALLOW_INSECURE_ENV`]=`1`.
///
/// # Errors
///
/// Returns an error describing how to opt in when the address is not loopback and the env is unset.
pub fn ensure_demo_bind_allowed(addr: SocketAddr) -> Result<(), String> {
    if addr.ip().is_loopback() {
        return Ok(());
    }
    match std::env::var(DEMO_ALLOW_INSECURE_ENV).as_deref() {
        Ok("1" | "true" | "TRUE" | "yes" | "YES") => Ok(()),
        _ => Err(format!(
            "E2E demo refuses non-loopback bind {addr} (insecure by design). \
             Use 127.0.0.1 or set {DEMO_ALLOW_INSECURE_ENV}=1 for lab-only exposure"
        )),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn loopback_allowed_without_env() {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 3000);
        ensure_demo_bind_allowed(addr).expect("loopback ok");
    }

    #[test]
    fn nonlocal_rejected_without_opt_in() {
        std::env::remove_var(DEMO_ALLOW_INSECURE_ENV);
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 3000);
        let err = ensure_demo_bind_allowed(addr).expect_err("public bind");
        assert!(err.contains(DEMO_ALLOW_INSECURE_ENV));
    }

    #[test]
    fn nonlocal_allowed_with_opt_in() {
        std::env::set_var(DEMO_ALLOW_INSECURE_ENV, "1");
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 3000);
        ensure_demo_bind_allowed(addr).expect("opt-in ok");
        std::env::remove_var(DEMO_ALLOW_INSECURE_ENV);
    }
}

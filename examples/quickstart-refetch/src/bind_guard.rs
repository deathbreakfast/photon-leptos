//! Loopback-only bind for teaching hosts.

use std::net::SocketAddr;

/// Refuse non-loopback binds (lab hosts use insecure Origin for demos).
pub fn ensure_loopback_bind(addr: SocketAddr) -> Result<(), String> {
    if addr.ip().is_loopback() {
        Ok(())
    } else {
        Err(format!(
            "quickstart-refetch refuses non-loopback bind {addr}; use 127.0.0.1"
        ))
    }
}

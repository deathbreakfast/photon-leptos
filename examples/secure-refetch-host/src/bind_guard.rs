//! Loopback-only bind.

use std::net::SocketAddr;

/// Refuse non-loopback binds.
pub fn ensure_loopback_bind(addr: SocketAddr) -> Result<(), String> {
    if addr.ip().is_loopback() {
        Ok(())
    } else {
        Err(format!(
            "secure-refetch-host refuses non-loopback bind {addr}; use 127.0.0.1"
        ))
    }
}

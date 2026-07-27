//! Loopback bind guard.

use std::net::SocketAddr;

/// Refuse non-loopback binds.
pub fn ensure_loopback_bind(addr: SocketAddr) -> Result<(), String> {
    if addr.ip().is_loopback() {
        Ok(())
    } else {
        Err(format!(
            "replace-and-append-demo refuses non-loopback bind {addr}"
        ))
    }
}

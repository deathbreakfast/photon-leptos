//! Resolve the Photon subscribe `key_filter` from auth mode and optional client key.
//!
//! # Policy
//!
//! | [`WsAuthMode`] | Client `key` (`?key=`) | Result |
//! |----------------|------------------------|--------|
//! | [`None`](WsAuthMode::None) | absent | `Ok(None)` — broadcast |
//! | [`None`](WsAuthMode::None) | present | `Ok(Some(key))` |
//! | [`User`](WsAuthMode::User) | absent | `Ok(Some(user_key))`, or [`MissingUser`](KeyResolveError::MissingUser) |
//! | [`User`](WsAuthMode::User) | present | `Ok(Some(key))` if `user_key == key`, else [`KeyMismatch`](KeyResolveError::KeyMismatch) |
//!
//! HTTP responses should use [`KeyResolveError::client_message`] so mismatch
//! bodies never reflect raw key identifiers.
//!
//! # Examples
//!
//! Broadcast (`auth = none`, no client key):
//!
//! ```
//! use photon_axum::axum_ws::key_resolve::resolve_subscribe_key;
//! use photon_axum::{KeyResolveError, WsAuthMode};
//!
//! let key = resolve_subscribe_key(WsAuthMode::None, None, None).unwrap();
//! assert_eq!(key, None);
//! ```
//!
//! Key-only (`auth = none` + client key):
//!
//! ```
//! use photon_axum::axum_ws::key_resolve::resolve_subscribe_key;
//! use photon_axum::WsAuthMode;
//!
//! let key = resolve_subscribe_key(
//!     WsAuthMode::None,
//!     None,
//!     Some("room-42"),
//! ).unwrap();
//! assert_eq!(key.as_deref(), Some("room-42"));
//! ```
//!
//! Auth-only (`auth = user`, no client key):
//!
//! ```
//! use photon_axum::axum_ws::key_resolve::resolve_subscribe_key;
//! use photon_axum::WsAuthMode;
//!
//! let key = resolve_subscribe_key(
//!     WsAuthMode::User,
//!     Some("1234"),
//!     None,
//! ).unwrap();
//! assert_eq!(key.as_deref(), Some("1234"));
//! ```
//!
//! Auth + key must match:
//!
//! ```
//! use photon_axum::axum_ws::key_resolve::resolve_subscribe_key;
//! use photon_axum::{KeyResolveError, WsAuthMode};
//!
//! let err = resolve_subscribe_key(
//!     WsAuthMode::User,
//!     Some("1234"),
//!     Some("1235"),
//! ).unwrap_err();
//! assert!(matches!(err, KeyResolveError::KeyMismatch { .. }));
//! ```

use thiserror::Error;

use super::descriptor::WsAuthMode;

/// Failure resolving a subscribe key for a WebSocket upgrade.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum KeyResolveError {
    /// `auth = "user"` but the extractor returned no user key.
    #[error("auth=user requires an authenticated user key")]
    MissingUser,
    /// Client `?key=` did not equal the authenticated `user_key`.
    ///
    /// Display / [`Self::client_message`] never include raw key material (SEC-001).
    #[error("key does not match authenticated scope")]
    KeyMismatch {
        /// Key from the host auth extractor.
        user_key: String,
        /// Key from the client query string.
        client_key: String,
    },
}

impl KeyResolveError {
    /// Client-facing HTTP body (never includes raw key material).
    #[must_use]
    pub fn client_message(&self) -> &'static str {
        match self {
            Self::MissingUser => "auth=user requires an authenticated user key",
            Self::KeyMismatch { .. } => "key does not match authenticated scope",
        }
    }
}

/// Resolve the Photon `key_filter` for a WS upgrade.
///
/// See the module-level policy table and examples.
pub fn resolve_subscribe_key(
    auth_mode: WsAuthMode,
    user_key: Option<&str>,
    client_key: Option<&str>,
) -> Result<Option<String>, KeyResolveError> {
    match auth_mode {
        WsAuthMode::None => Ok(client_key.filter(|k| !k.is_empty()).map(str::to_owned)),
        WsAuthMode::User => resolve_user_mode(user_key, client_key),
    }
}

fn resolve_user_mode(
    user_key: Option<&str>,
    client_key: Option<&str>,
) -> Result<Option<String>, KeyResolveError> {
    let Some(user) = user_key.filter(|k| !k.is_empty()) else {
        return Err(KeyResolveError::MissingUser);
    };

    match client_key.filter(|k| !k.is_empty()) {
        None => Ok(Some(user.to_owned())),
        Some(client) if client == user => Ok(Some(user.to_owned())),
        Some(client) => Err(KeyResolveError::KeyMismatch {
            user_key: user.to_owned(),
            client_key: client.to_owned(),
        }),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn none_without_client_key_is_broadcast() {
        assert_eq!(
            resolve_subscribe_key(WsAuthMode::None, None, None).unwrap(),
            None
        );
    }

    #[test]
    fn none_with_client_key_scopes() {
        assert_eq!(
            resolve_subscribe_key(WsAuthMode::None, Some("ignored"), Some("k1")).unwrap(),
            Some("k1".into())
        );
    }

    #[test]
    fn user_without_client_uses_user_key() {
        assert_eq!(
            resolve_subscribe_key(WsAuthMode::User, Some("1234"), None).unwrap(),
            Some("1234".into())
        );
    }

    #[test]
    fn user_missing_identity_errors() {
        assert_eq!(
            resolve_subscribe_key(WsAuthMode::User, None, None).unwrap_err(),
            KeyResolveError::MissingUser
        );
        assert_eq!(
            resolve_subscribe_key(WsAuthMode::User, Some(""), None).unwrap_err(),
            KeyResolveError::MissingUser
        );
    }

    #[test]
    fn user_with_matching_client_key_ok() {
        assert_eq!(
            resolve_subscribe_key(WsAuthMode::User, Some("1234"), Some("1234")).unwrap(),
            Some("1234".into())
        );
    }

    #[test]
    fn user_with_mismatched_client_key_errors() {
        let err = resolve_subscribe_key(WsAuthMode::User, Some("1234"), Some("1235")).unwrap_err();
        assert_eq!(
            err,
            KeyResolveError::KeyMismatch {
                user_key: "1234".into(),
                client_key: "1235".into(),
            }
        );
        let msg = err.to_string();
        assert!(!msg.contains("1234") && !msg.contains("1235"), "{msg}");
        assert_eq!(
            err.client_message(),
            "key does not match authenticated scope"
        );
    }
}

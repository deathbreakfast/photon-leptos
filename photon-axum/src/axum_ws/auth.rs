//! Auth extraction for `apply_ws_routes`.

use axum::extract::FromRequestParts;
use axum::http::request::Parts;

/// Extracts an optional user key from an Axum request extractor.
///
/// Implement this on a newtype wrapping your host auth session to support
/// `auth = "user"` WebSocket routes.
pub trait PhotonUserExtractor: Send + 'static {
    /// Return `Some(key)` for an authenticated user, `None` otherwise.
    fn user_key(&self) -> Option<String>;
}

/// Placeholder auth for headless runtimes: always succeeds extraction and provides no user key.
///
/// Use with [`super::apply_ws_routes`] when the process does not host an auth/session stack.
#[derive(Clone, Copy, Debug, Default)]
pub struct HeadlessWsAuth;

impl PhotonUserExtractor for HeadlessWsAuth {
    fn user_key(&self) -> Option<String> {
        None
    }
}

impl<S> FromRequestParts<S> for HeadlessWsAuth
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(_parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(HeadlessWsAuth)
    }
}

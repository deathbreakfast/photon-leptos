//! E2E cookie-based user extractor for `auth = "user"` WebSocket routes.

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::StatusCode;
use photon_axum::PhotonUserExtractor;

/// Cookie name holding the demo user id (`e2e_user`).
pub const E2E_USER_COOKIE: &str = "e2e_user";

/// Reads [`E2E_USER_COOKIE`] and exposes it as [`PhotonUserExtractor::user_key`].
///
/// Extraction always succeeds (including when the cookie is missing) so `auth = "none"`
/// routes are unaffected. Missing identity surfaces as `None` from
/// [`PhotonUserExtractor::user_key`].
#[derive(Clone, Debug, Default)]
pub struct E2eUserAuth {
    user_key: Option<String>,
}

impl PhotonUserExtractor for E2eUserAuth {
    fn user_key(&self) -> Option<String> {
        self.user_key.clone()
    }
}

impl<S> FromRequestParts<S> for E2eUserAuth
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(Self {
            user_key: cookie_value(&parts.headers, E2E_USER_COOKIE),
        })
    }
}

/// Parse a cookie value from the `Cookie` header (first match).
pub fn cookie_value(headers: &axum::http::HeaderMap, name: &str) -> Option<String> {
    let cookie = headers.get(axum::http::header::COOKIE)?.to_str().ok()?;
    for part in cookie.split(';') {
        let part = part.trim();
        if let Some(value) = part
            .strip_prefix(name)
            .and_then(|rest| rest.strip_prefix('='))
        {
            if value.is_empty() {
                return None;
            }
            return Some(value.to_string());
        }
    }
    None
}

/// Rejection helper for tests that need a hard 401 (unused by [`E2eUserAuth`]).
#[allow(dead_code)]
pub fn unauthorized() -> StatusCode {
    StatusCode::UNAUTHORIZED
}

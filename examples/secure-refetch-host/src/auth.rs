//! Session cookie → `PhotonUserExtractor`.

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use photon_axum::PhotonUserExtractor;

/// Session cookie name (demo; production should set Secure + SameSite).
pub const SESSION_COOKIE: &str = "demo_session";

/// Reads [`SESSION_COOKIE`] as the Photon user key.
#[derive(Clone, Debug, Default)]
pub struct SessionUserAuth {
    user_key: Option<String>,
}

impl PhotonUserExtractor for SessionUserAuth {
    fn user_key(&self) -> Option<String> {
        self.user_key.clone()
    }
}

impl<S> FromRequestParts<S> for SessionUserAuth
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(Self {
            user_key: cookie_value(&parts.headers, SESSION_COOKIE),
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

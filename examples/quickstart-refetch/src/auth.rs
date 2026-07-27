//! Cookie-based user extractor for `auth = "user"` routes.

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use photon_axum::PhotonUserExtractor;

/// Cookie name holding the demo user id.
pub const DEMO_USER_COOKIE: &str = "demo_user";

/// Reads [`DEMO_USER_COOKIE`] → [`PhotonUserExtractor::user_key`].
#[derive(Clone, Debug, Default)]
pub struct DemoUserAuth {
    user_key: Option<String>,
}

impl PhotonUserExtractor for DemoUserAuth {
    fn user_key(&self) -> Option<String> {
        self.user_key.clone()
    }
}

impl<S> FromRequestParts<S> for DemoUserAuth
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(Self {
            user_key: cookie_value(&parts.headers, DEMO_USER_COOKIE),
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

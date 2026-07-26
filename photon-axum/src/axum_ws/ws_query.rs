//! Extract optional subscribe key from a WebSocket upgrade request URI.

use std::borrow::Cow;

use axum::http::Uri;

/// Query parameter name for the client subscribe key.
pub const KEY_QUERY_PARAM: &str = "key";

/// Maximum accepted subscribe key length in UTF-8 bytes.
pub const MAX_KEY_LEN: usize = 256;

/// Parse `?key=` (or `&key=`) from a request [`Uri`].
///
/// Empty or missing values yield [`None`]. Malformed percent-encoding, invalid
/// UTF-8 after decode, keys longer than [`MAX_KEY_LEN`], or keys containing
/// disallowed control characters (`\0`, `\r`, `\n`, and other Unicode controls)
/// are rejected as [`None`] (callers treat as absent / invalid — prefer failing
/// auth over corrupting the partition key).
///
/// # Examples
///
/// ```
/// use axum::http::Uri;
/// use photon_axum::axum_ws::ws_query::client_key_from_uri;
///
/// let uri: Uri = "/ws/notifications?key=1234".parse().unwrap();
/// assert_eq!(client_key_from_uri(&uri).as_deref(), Some("1234"));
///
/// let bare: Uri = "/ws/notifications".parse().unwrap();
/// assert_eq!(client_key_from_uri(&bare), None);
/// ```
pub fn client_key_from_uri(uri: &Uri) -> Option<String> {
    let query = uri.query()?;
    for pair in query.split('&') {
        let mut parts = pair.splitn(2, '=');
        let name = parts.next()?;
        if name != KEY_QUERY_PARAM {
            continue;
        }
        let value = parts.next().unwrap_or("");
        let decoded = percent_decode_utf8(value)?;
        if decoded.is_empty() {
            return None;
        }
        if decoded.len() > MAX_KEY_LEN {
            return None;
        }
        if key_has_disallowed_control_chars(&decoded) {
            return None;
        }
        return Some(decoded);
    }
    None
}

/// Whether `key` contains characters that must not appear in subscribe keys.
///
/// Rejects NUL, CR, LF, and any other Unicode control character.
#[must_use]
pub fn key_has_disallowed_control_chars(key: &str) -> bool {
    key.chars().any(is_disallowed_key_control)
}

/// Escape control characters in a subscribe key before placing it in ops logs.
///
/// Client keys are rejected at parse time; this still covers host-supplied
/// `user_key` values and defense-in-depth for hub logging.
#[must_use]
pub fn sanitize_key_for_ops_log(key: &str) -> Cow<'_, str> {
    if !key_has_disallowed_control_chars(key) {
        return Cow::Borrowed(key);
    }
    Cow::Owned(
        key.chars()
            .map(|c| {
                if is_disallowed_key_control(c) {
                    '\u{FFFD}'
                } else {
                    c
                }
            })
            .collect(),
    )
}

fn is_disallowed_key_control(c: char) -> bool {
    // Includes `\0`, `\r`, `\n`, and other Unicode controls.
    c.is_control()
}

/// Decode a single `application/x-www-form-urlencoded` value to UTF-8.
///
/// Returns [`None`] on malformed `%` sequences or invalid UTF-8.
fn percent_decode_utf8(input: &str) -> Option<String> {
    let mut out = Vec::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' => {
                if i + 2 >= bytes.len() {
                    return None;
                }
                let hi = from_hex(bytes[i + 1])?;
                let lo = from_hex(bytes[i + 2])?;
                out.push(hi * 16 + lo);
                i += 3;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8(out).ok()
}

fn from_hex(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn extracts_key_query() {
        let uri: Uri = "/ws/x?key=abc".parse().unwrap();
        assert_eq!(client_key_from_uri(&uri).as_deref(), Some("abc"));
    }

    #[test]
    fn extracts_key_among_other_params() {
        let uri: Uri = "/ws/x?ns=w0&key=1234&x=1".parse().unwrap();
        assert_eq!(client_key_from_uri(&uri).as_deref(), Some("1234"));
    }

    #[test]
    fn empty_key_is_none() {
        let uri: Uri = "/ws/x?key=".parse().unwrap();
        assert_eq!(client_key_from_uri(&uri), None);
    }

    #[test]
    fn percent_decodes_key() {
        let uri: Uri = "/ws/x?key=a%2Fb".parse().unwrap();
        assert_eq!(client_key_from_uri(&uri).as_deref(), Some("a/b"));
    }

    #[test]
    fn percent_decodes_utf8_multibyte() {
        // é = C3 A9
        let uri: Uri = "/ws/x?key=%C3%A9".parse().unwrap();
        assert_eq!(client_key_from_uri(&uri).as_deref(), Some("é"));
    }

    #[test]
    fn percent_decodes_emoji() {
        // 😀 = F0 9F 98 80
        let uri: Uri = "/ws/x?key=%F0%9F%98%80".parse().unwrap();
        assert_eq!(client_key_from_uri(&uri).as_deref(), Some("😀"));
    }

    #[test]
    fn percent_decodes_cjk() {
        // 日 = E6 97 A5
        let uri: Uri = "/ws/x?key=%E6%97%A5".parse().unwrap();
        assert_eq!(client_key_from_uri(&uri).as_deref(), Some("日"));
    }

    #[test]
    fn plus_is_space() {
        let uri: Uri = "/ws/x?key=a+b".parse().unwrap();
        assert_eq!(client_key_from_uri(&uri).as_deref(), Some("a b"));
    }

    #[test]
    fn rejects_malformed_percent() {
        let uri: Uri = "/ws/x?key=%ZZ".parse().unwrap();
        assert_eq!(client_key_from_uri(&uri), None);
        let truncated: Uri = "/ws/x?key=%C3".parse().unwrap();
        assert_eq!(client_key_from_uri(&truncated), None);
    }

    #[test]
    fn rejects_invalid_utf8_after_decode() {
        // Lone continuation byte — valid hex decode, invalid UTF-8.
        let uri: Uri = "/ws/x?key=%80".parse().unwrap();
        assert_eq!(client_key_from_uri(&uri), None);
    }

    #[test]
    fn rejects_oversized_key() {
        let long = "a".repeat(MAX_KEY_LEN + 1);
        let uri: Uri = format!("/ws/x?key={long}").parse().unwrap();
        assert_eq!(client_key_from_uri(&uri), None);
    }

    #[test]
    fn accepts_max_len_key() {
        let exact = "a".repeat(MAX_KEY_LEN);
        let uri: Uri = format!("/ws/x?key={exact}").parse().unwrap();
        assert_eq!(client_key_from_uri(&uri).as_deref(), Some(exact.as_str()));
    }

    #[test]
    fn rejects_nul_in_key() {
        let uri: Uri = "/ws/x?key=a%00b".parse().unwrap();
        assert_eq!(client_key_from_uri(&uri), None);
    }

    #[test]
    fn rejects_cr_lf_in_key() {
        let cr: Uri = "/ws/x?key=a%0Db".parse().unwrap();
        assert_eq!(client_key_from_uri(&cr), None);
        let lf: Uri = "/ws/x?key=a%0Ab".parse().unwrap();
        assert_eq!(client_key_from_uri(&lf), None);
        let crlf: Uri = "/ws/x?key=a%0D%0Ab".parse().unwrap();
        assert_eq!(client_key_from_uri(&crlf), None);
    }

    #[test]
    fn rejects_other_ascii_controls() {
        // TAB
        let tab: Uri = "/ws/x?key=a%09b".parse().unwrap();
        assert_eq!(client_key_from_uri(&tab), None);
        // BEL
        let bel: Uri = "/ws/x?key=%07".parse().unwrap();
        assert_eq!(client_key_from_uri(&bel), None);
    }

    #[test]
    fn sanitize_ops_log_replaces_controls() {
        let dirty = "user\nname\0";
        let cleaned = sanitize_key_for_ops_log(dirty);
        assert!(!cleaned.contains('\n'));
        assert!(!cleaned.contains('\0'));
        assert!(cleaned.contains('\u{FFFD}'));
        assert_eq!(sanitize_key_for_ops_log("clean-key"), "clean-key");
    }
}

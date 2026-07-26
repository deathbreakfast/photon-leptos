//! Build WebSocket paths that carry an optional Photon subscribe key.

/// Path (no query) and whether a `key` query parameter is present.
///
/// Use for logs — never emit the raw `?key=` value.
#[must_use]
pub fn ws_url_log_fields(url: &str) -> (&str, bool) {
    let (path, query) = match url.split_once('?') {
        Some((path, query)) => (path, Some(query)),
        None => (url, None),
    };
    let has_key = query.is_some_and(|q| {
        q.split('&').any(|pair| {
            let name = pair.split('=').next().unwrap_or("");
            name == "key"
        })
    });
    (path, has_key)
}

/// Append `?key=` / `&key=` to a WebSocket path when a key filter is set.
///
/// The server reads this query param and applies the auth + key policy.
///
/// # Examples
///
/// ```
/// use photon_leptos::ws_url_with_key;
///
/// assert_eq!(
///     ws_url_with_key("/ws/notifications", None),
///     "/ws/notifications"
/// );
/// assert_eq!(
///     ws_url_with_key("/ws/notifications", Some("1234")),
///     "/ws/notifications?key=1234"
/// );
/// assert_eq!(
///     ws_url_with_key("/ws/notifications?ns=w0", Some("1234")),
///     "/ws/notifications?ns=w0&key=1234"
/// );
/// ```
pub fn ws_url_with_key(ws_path: &str, key_filter: Option<&str>) -> String {
    let Some(key) = key_filter.filter(|k| !k.is_empty()) else {
        return ws_path.to_owned();
    };
    let encoded = urlencoding_encode(key);
    if ws_path.contains('?') {
        format!("{ws_path}&key={encoded}")
    } else {
        format!("{ws_path}?key={encoded}")
    }
}

/// Encode a key for a query value (unreserved chars left alone).
fn urlencoding_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for b in input.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(char::from(b));
            }
            _ => {
                out.push('%');
                out.push(hex_digit(b >> 4));
                out.push(hex_digit(b & 0x0f));
            }
        }
    }
    out
}

fn hex_digit(n: u8) -> char {
    char::from(if n < 10 { b'0' + n } else { b'A' + (n - 10) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn omits_empty_key() {
        assert_eq!(ws_url_with_key("/ws/x", Some("")), "/ws/x");
        assert_eq!(ws_url_with_key("/ws/x", None), "/ws/x");
    }

    #[test]
    fn adds_query_or_ampersand() {
        assert_eq!(ws_url_with_key("/ws/x", Some("a")), "/ws/x?key=a");
        assert_eq!(ws_url_with_key("/ws/x?y=1", Some("a")), "/ws/x?y=1&key=a");
    }

    #[test]
    fn encodes_reserved() {
        assert_eq!(ws_url_with_key("/ws/x", Some("a/b")), "/ws/x?key=a%2Fb");
    }

    #[test]
    fn encodes_utf8_multibyte() {
        assert_eq!(ws_url_with_key("/ws/x", Some("é")), "/ws/x?key=%C3%A9");
        assert_eq!(
            ws_url_with_key("/ws/x", Some("😀")),
            "/ws/x?key=%F0%9F%98%80"
        );
    }

    #[test]
    fn log_fields_redact_key_value() {
        let url = ws_url_with_key("/ws/notifications", Some("secret-partition"));
        let (path, has_key) = ws_url_log_fields(&url);
        assert_eq!(path, "/ws/notifications");
        assert!(has_key);
        assert!(!path.contains("secret"));
        assert!(!path.contains("key="));

        let (bare, no_key) = ws_url_log_fields("/ws/notifications");
        assert_eq!(bare, "/ws/notifications");
        assert!(!no_key);

        let (with_other, keyed) = ws_url_log_fields("/ws/x?ns=w0&key=abc");
        assert_eq!(with_other, "/ws/x");
        assert!(keyed);
    }
}

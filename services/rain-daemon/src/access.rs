//! The daemon is loopback-only. Remote gateways must authenticate users and inject
//! a server-side secret after stripping any client-supplied X-Rain-Gateway-Token.
use hyper::{HeaderMap, Method};

pub(crate) fn allowed(
    headers: &HeaderMap,
    method: &Method,
    gateway_token: Option<&str>,
    origins: &str,
) -> bool {
    let value = |name: &str| headers.get(name).and_then(|v| v.to_str().ok());
    let host = value("host").unwrap_or("");
    let local_host = host == "localhost"
        || host.starts_with("localhost:")
        || host == "127.0.0.1"
        || host.starts_with("127.0.0.1:")
        || host == "[::1]"
        || host.starts_with("[::1]:");
    let proxied = headers.contains_key("forwarded")
        || headers.contains_key("x-forwarded-for")
        || headers.contains_key("x-forwarded-host")
        || headers.contains_key("x-forwarded-proto");
    let gateway = gateway_token
        .filter(|s| s.len() >= 32)
        .map(|secret| {
            value("x-rain-gateway-token")
                .map(|supplied| constant_time_eq(secret.as_bytes(), supplied.as_bytes()))
                .unwrap_or(false)
        })
        .unwrap_or(false);
    if (!local_host || proxied) && !gateway {
        return false;
    }
    if let Some(origin) = value("origin") {
        let local_origin = ["http://localhost", "http://127.0.0.1", "http://[::1]"]
            .iter()
            .any(|base| {
                origin == *base
                    || origin
                        .strip_prefix(base)
                        .map(|s| {
                            s.strip_prefix(':')
                                .map(|port| {
                                    !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit())
                                })
                                .unwrap_or(false)
                        })
                        .unwrap_or(false)
            });
        if !(local_host && !proxied && local_origin)
            && !origins
                .split(',')
                .any(|o| !o.trim().is_empty() && o.trim() == origin)
        {
            return false;
        }
    } else if headers.get("sec-fetch-site").and_then(|h| h.to_str().ok()) == Some("cross-site") {
        return false;
    }
    // All methods, including preflight and static resources, pass the same boundary.
    let _ = method;
    true
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |diff, (a, b)| diff | (a ^ b)) == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    fn headers(host: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert("host", host.parse().unwrap());
        h
    }
    #[test]
    fn gateway_is_closed_without_secret() {
        for host in ["evil.example", "localhost.evil.example", "hub.private"] {
            assert!(!allowed(&headers(host), &Method::GET, None, ""));
        }
        let mut h = headers("127.0.0.1:7890");
        assert!(allowed(&h, &Method::GET, None, ""));
        h.insert("x-forwarded-for", "100.64.0.2".parse().unwrap());
        assert!(!allowed(&h, &Method::GET, None, ""));
    }
    #[test]
    fn gateway_requires_secret_and_exact_origin() {
        let token = "a".repeat(64);
        let mut h = headers("hub.private");
        h.insert("x-rain-gateway-token", token.parse().unwrap());
        h.insert("origin", "https://hub.private".parse().unwrap());
        assert!(allowed(
            &h,
            &Method::POST,
            Some(&token),
            "https://hub.private"
        ));
        assert!(!allowed(
            &h,
            &Method::POST,
            Some(&token),
            "https://other.private"
        ));
        assert!(!allowed(
            &h,
            &Method::POST,
            Some("wrong"),
            "https://hub.private"
        ));
    }
    #[test]
    fn local_browser_cannot_cross_boundary() {
        let mut h = headers("localhost:7890");
        h.insert("origin", "https://evil.example".parse().unwrap());
        assert!(!allowed(&h, &Method::POST, None, ""));
        h.insert("origin", "http://localhost:5173".parse().unwrap());
        assert!(allowed(&h, &Method::POST, None, ""));
    }
}

use crate::error::{AppError, Result};
use ipnet::IpNet;
use reqwest::{StatusCode, Url, redirect::Policy};
use serde::Deserialize;
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::LazyLock;
use std::time::Duration;

const MAX_RESPONSE_BYTES: usize = 1_048_576;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

static NON_PUBLIC_NETWORKS: LazyLock<Vec<IpNet>> = LazyLock::new(|| {
    [
        "0.0.0.0/8",
        "10.0.0.0/8",
        "100.64.0.0/10",
        "127.0.0.0/8",
        "169.254.0.0/16",
        "172.16.0.0/12",
        "192.0.0.0/24",
        "192.0.2.0/24",
        "192.88.99.0/24",
        "192.168.0.0/16",
        "198.18.0.0/15",
        "198.51.100.0/24",
        "203.0.113.0/24",
        "224.0.0.0/4",
        "240.0.0.0/4",
        "::/128",
        "::1/128",
        "::ffff:0:0/96",
        "64:ff9b:1::/48",
        "100::/64",
        "2001::/23",
        "2001:db8::/32",
        "2002::/16",
        "fc00::/7",
        "fe80::/10",
        "ff00::/8",
    ]
    .iter()
    .map(|cidr| cidr.parse().expect("static IP prefix must be valid"))
    .collect()
});

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApprovedPreview {
    pub origin: String,
    #[serde(default)]
    pub allow_private: bool,
}

#[derive(Clone, Debug, Default)]
pub struct E2eProbePolicy {
    origins: HashMap<String, Vec<ApprovedPreview>>,
}

pub struct ProbePage {
    pub status: StatusCode,
    pub html: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ProbeError {
    #[error("Security Error: E2E destination not authorized")]
    Unauthorized,
    #[error("E2E probe failed: {0}")]
    Network(String),
    #[error("E2E probe rejected an oversized response")]
    ResponseTooLarge,
    #[error("E2E probe cannot follow redirects")]
    Redirect,
}

impl E2eProbePolicy {
    pub fn from_json(json: &str) -> Result<Self> {
        let raw: HashMap<String, Vec<ApprovedPreview>> = serde_json::from_str(json)
            .map_err(|error| AppError::Config(format!("Invalid E2E_ALLOWED_PREVIEWS: {error}")))?;
        let mut origins = HashMap::new();

        for (repo, entries) in raw {
            let mut names = repo.split('/');
            if names.next().is_none_or(str::is_empty)
                || names.next().is_none_or(str::is_empty)
                || names.next().is_some()
                || repo.chars().any(char::is_whitespace)
            {
                return Err(AppError::Config(
                    "E2E_ALLOWED_PREVIEWS must use exact owner/repo keys".into(),
                ));
            }
            let mut validated = Vec::new();
            for mut entry in entries {
                let url = Url::parse(&entry.origin).map_err(|_| {
                    AppError::Config("E2E preview origin is not a valid URL".into())
                })?;
                let local = is_explicit_loopback_origin(&url);
                if !url.username().is_empty()
                    || url.password().is_some()
                    || url.query().is_some()
                    || url.fragment().is_some()
                    || url.path() != "/"
                    || !matches!(url.scheme(), "https" | "http")
                    || (url.scheme() == "http" && !(local && entry.allow_private))
                    || (entry.allow_private && !local)
                {
                    return Err(AppError::Config(
                        "E2E preview must be an HTTPS origin, or an explicitly authorized HTTP loopback origin".into(),
                    ));
                }
                entry.origin = url.origin().ascii_serialization();
                validated.push(entry);
            }
            if origins
                .insert(repo.to_ascii_lowercase(), validated)
                .is_some()
            {
                return Err(AppError::Config("Duplicate E2E repository policy".into()));
            }
        }
        Ok(Self { origins })
    }

    pub async fn fetch(
        &self,
        repo: &str,
        raw_url: &str,
    ) -> std::result::Result<ProbePage, ProbeError> {
        // Permission checks intentionally precede DNS and socket creation.
        let requested = Url::parse(raw_url).map_err(|_| ProbeError::Unauthorized)?;
        if !requested.username().is_empty()
            || requested.password().is_some()
            || !matches!(requested.scheme(), "https" | "http")
        {
            return Err(ProbeError::Unauthorized);
        }
        let allowed = self
            .origins
            .get(&repo.to_ascii_lowercase())
            .ok_or(ProbeError::Unauthorized)?
            .iter()
            .find(|entry| entry.origin == requested.origin().ascii_serialization())
            .ok_or(ProbeError::Unauthorized)?;
        if requested.scheme() == "http" && !allowed.allow_private {
            return Err(ProbeError::Unauthorized);
        }

        let host = requested.host_str().ok_or(ProbeError::Unauthorized)?;
        let port = requested
            .port_or_known_default()
            .ok_or(ProbeError::Unauthorized)?;
        let host_ip = host
            .trim_start_matches('[')
            .trim_end_matches(']')
            .parse::<IpAddr>()
            .ok();
        let addresses: Vec<SocketAddr> = if let Some(ip) = host_ip {
            vec![SocketAddr::new(ip, port)]
        } else {
            tokio::time::timeout(CONNECT_TIMEOUT, tokio::net::lookup_host((host, port)))
                .await
                .map_err(|_| ProbeError::Network("DNS lookup timed out".into()))?
                .map_err(|_| ProbeError::Network("DNS lookup failed".into()))?
                .collect()
        };
        if addresses.is_empty()
            || addresses.iter().any(|address| {
                if allowed.allow_private {
                    !address.ip().is_loopback()
                } else {
                    !is_public_address(address.ip())
                }
            })
        {
            return Err(ProbeError::Unauthorized);
        }

        // Pin the DNS decision to the actual connection, and never follow a
        // redirect or environment-provided proxy across the trust boundary.
        let mut client = reqwest::Client::builder()
            .no_proxy()
            .redirect(Policy::none())
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT);
        if host_ip.is_none() {
            client = client.resolve_to_addrs(host, &addresses);
        }
        let client = client
            .build()
            .map_err(|error| ProbeError::Network(error.to_string()))?;
        let mut response = client
            .get(requested)
            .send()
            .await
            .map_err(|error| ProbeError::Network(error.to_string()))?;
        if response.status().is_redirection() {
            return Err(ProbeError::Redirect);
        }
        if response
            .content_length()
            .is_some_and(|n| n > MAX_RESPONSE_BYTES as u64)
        {
            return Err(ProbeError::ResponseTooLarge);
        }

        let status = response.status();
        let mut body = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|error| ProbeError::Network(error.to_string()))?
        {
            if chunk.len() > MAX_RESPONSE_BYTES.saturating_sub(body.len()) {
                return Err(ProbeError::ResponseTooLarge);
            }
            body.extend_from_slice(&chunk);
        }
        Ok(ProbePage {
            status,
            html: String::from_utf8_lossy(&body).into_owned(),
        })
    }
}

fn is_explicit_loopback_origin(url: &Url) -> bool {
    url.host_str().is_some_and(|host| {
        host.eq_ignore_ascii_case("localhost")
            || host
                .trim_start_matches('[')
                .trim_end_matches(']')
                .parse::<IpAddr>()
                .is_ok_and(|ip| ip.is_loopback())
    })
}

fn is_public_address(ip: IpAddr) -> bool {
    // DNS64 may encode an IPv4 destination in the well-known NAT64 prefix.
    // Apply the same policy to that destination while allowing public IPv4.
    if let IpAddr::V6(address) = ip {
        let octets = address.octets();
        if octets[..12] == [0x00, 0x64, 0xff, 0x9b, 0, 0, 0, 0, 0, 0, 0, 0] {
            return is_public_address(IpAddr::V4(std::net::Ipv4Addr::new(
                octets[12], octets[13], octets[14], octets[15],
            )));
        }
    }
    !NON_PUBLIC_NETWORKS
        .iter()
        .any(|network| network.contains(&ip))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nat64_addresses_preserve_embedded_ipv4_policy() {
        for address in [
            "64:ff9b::a00:1",
            "64:ff9b::7f00:1",
            "64:ff9b::a9fe:a9fe",
            "64:ff9b::c0a8:1",
            "64:ff9b::c000:201",
        ] {
            assert!(!is_public_address(address.parse().unwrap()), "{address}");
        }
        assert!(is_public_address("64:ff9b::808:808".parse().unwrap()));
    }

    #[test]
    fn repository_policy_rejects_whitespace_in_keys() {
        for repo in [
            "owner/repo ",
            " owner/repo",
            "own er/repo",
            "owner/re\tpo",
            "owner/repo\u{a0}",
        ] {
            let config = serde_json::json!({repo: [{"origin": "https://preview.example.org"}]});
            assert!(
                E2eProbePolicy::from_json(&config.to_string()).is_err(),
                "{repo}"
            );
        }
    }

    #[test]
    fn rejects_unscoped_or_untrusted_preview_configuration() {
        assert!(
            E2eProbePolicy::from_json(
                r#"{"owner/repo":[{"origin":"https://preview.example.org"}]}"#
            )
            .is_ok()
        );
        for invalid in [
            r#"{"owner/repo":[{"origin":"http://example.org"}]}"#,
            r#"{"owner/repo":[{"origin":"https://example.org/path"}]}"#,
            r#"{"owner/repo":[{"origin":"https://example.org","allow_private":true}]}"#,
            r#"{"owner/repo":[{"origin":"http://127.0.0.1:3000"}]}"#,
            r#"{"owner/repo":[{"origin":"https://user:pass@example.org"}]}"#,
            r#"{"all":[{"origin":"https://example.org"}]}"#,
        ] {
            assert!(E2eProbePolicy::from_json(invalid).is_err(), "{invalid}");
        }
    }

    #[test]
    fn public_network_checks_reject_sensitive_and_reserved_addresses() {
        for address in [
            "127.0.0.1",
            "10.0.0.1",
            "169.254.169.254",
            "192.168.10.1",
            "172.16.0.1",
            "100.64.0.1",
            "198.18.0.1",
            "192.0.2.1",
            "203.0.113.1",
            "::1",
            "::ffff:127.0.0.1",
            "fe80::1",
            "fc00::1",
        ] {
            assert!(!is_public_address(address.parse().unwrap()), "{address}");
        }
        assert!(is_public_address("8.8.8.8".parse().unwrap()));
        assert!(is_public_address("2606:4700:4700::1111".parse().unwrap()));
    }

    #[tokio::test]
    async fn repository_policy_rejects_other_repo_before_dns() {
        let policy = E2eProbePolicy::from_json(
            r#"{"owner/allowed":[{"origin":"https://preview.example.org"}]}"#,
        )
        .unwrap();
        assert!(matches!(
            policy
                .fetch("owner/other", "https://preview.example.org/")
                .await,
            Err(ProbeError::Unauthorized)
        ));
        assert!(matches!(
            policy
                .fetch("owner/allowed", "https://unapproved.invalid/")
                .await,
            Err(ProbeError::Unauthorized)
        ));
    }

    #[tokio::test]
    async fn explicit_loopback_preview_is_scoped_and_redirects_are_not_followed() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let policy = E2eProbePolicy::from_json(&format!(
            r#"{{"owner/repo":[{{"origin":"http://{address}","allow_private":true}}]}}"#
        ))
        .unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = [0u8; 2048];
            let _ = stream.read(&mut request).await;
            stream.write_all(b"HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:1/\r\nContent-Length: 0\r\n\r\n").await.unwrap();
        });
        assert!(matches!(
            policy
                .fetch("owner/repo", &format!("http://{address}/"))
                .await,
            Err(ProbeError::Redirect)
        ));
        server.await.unwrap();
    }

    #[tokio::test]
    async fn explicitly_approved_loopback_origin_returns_html() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let policy = E2eProbePolicy::from_json(&format!(
            r#"{{"owner/repo":[{{"origin":"http://{address}","allow_private":true}}]}}"#
        ))
        .unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = [0u8; 2048];
            let _ = stream.read(&mut request).await;
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 11\r\n\r\n<h1>ok</h1>")
                .await
                .unwrap();
        });
        let page = policy
            .fetch("owner/repo", &format!("http://{address}/index.html"))
            .await
            .unwrap();
        assert_eq!(page.status, StatusCode::OK);
        assert_eq!(page.html, "<h1>ok</h1>");
        server.await.unwrap();
    }

    #[tokio::test]
    async fn oversized_preview_is_rejected_before_reading_body() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let policy = E2eProbePolicy::from_json(&format!(
            r#"{{"owner/repo":[{{"origin":"http://{address}","allow_private":true}}]}}"#
        ))
        .unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = [0u8; 2048];
            let _ = stream.read(&mut request).await;
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 1048577\r\n\r\n")
                .await
                .unwrap();
        });
        assert!(matches!(
            policy
                .fetch("owner/repo", &format!("http://{address}/"))
                .await,
            Err(ProbeError::ResponseTooLarge)
        ));
        server.await.unwrap();
    }
}

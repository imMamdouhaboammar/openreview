use crate::error::{AppError, Result};
use reqwest::Url;

/// Generate a GitHub-owned registration form with repository permissions and
/// webhook events preselected. No App credentials or registration codes ever
/// flow through OpenReview: the operator completes the form at github.com.
pub fn github_app_registration_url(
    domain: &str,
    app_name: &str,
    owner_org: Option<&str>,
    public: bool,
) -> Result<String> {
    let site = Url::parse(domain)
        .map_err(|_| AppError::Config("--domain must be a valid HTTPS origin".into()))?;
    let host = match site.host_str() {
        Some(host) if host.contains('.') && host.parse::<std::net::IpAddr>().is_err() => host,
        _ => {
            return Err(AppError::Config(
                "--domain must use a publicly reachable DNS hostname".into(),
            ));
        }
    };
    if site.scheme() != "https"
        || !site.username().is_empty()
        || site.password().is_some()
        || site.path() != "/"
        || site.query().is_some()
        || site.fragment().is_some()
        || host.eq_ignore_ascii_case("your_domain")
        || host.starts_with("localhost.")
    {
        return Err(AppError::Config(
            "--domain must be a public HTTPS origin without credentials, path, query or fragment"
                .into(),
        ));
    }
    let name = app_name.trim();
    if name.is_empty() || name.len() > 100 {
        return Err(AppError::Config(
            "--name must contain 1 to 100 characters".into(),
        ));
    }
    let mut registration = Url::parse("https://github.com/settings/apps/new")
        .expect("static GitHub registration URL is valid");
    if let Some(org) = owner_org {
        let valid_org = !org.is_empty()
            && org.len() <= 39
            && !org.starts_with('-')
            && !org.ends_with('-')
            && org.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-');
        if !valid_org {
            return Err(AppError::Config(
                "--org must be a GitHub organization name (letters, digits or internal hyphens)"
                    .into(),
            ));
        }
        registration.set_path(&format!("/organizations/{org}/settings/apps/new"));
    }
    let webhook_url = site
        .join("api/webhooks/github")
        .map_err(|_| AppError::Config("Could not construct the GitHub webhook endpoint".into()))?;
    {
        let mut pairs = registration.query_pairs_mut();
        pairs.append_pair("name", name);
        pairs.append_pair(
            "description",
            "Self-hosted code review and authorized fix agent",
        );
        pairs.append_pair("url", site.as_str());
        pairs.append_pair("webhook_active", "true");
        pairs.append_pair("webhook_url", webhook_url.as_str());
        pairs.append_pair("public", if public { "true" } else { "false" });
        for (permission, level) in [
            ("contents", "write"),
            ("issues", "write"),
            ("pull_requests", "write"),
        ] {
            pairs.append_pair(permission, level);
        }
        for event in [
            "issue_comment",
            "pull_request",
            "pull_request_review_comment",
            "installation",
            "installation_repositories",
        ] {
            pairs.append_pair("events[]", event);
        }
    }
    Ok(registration.to_string())
}

#[cfg(test)]
mod tests {
    use super::github_app_registration_url;
    use reqwest::Url;

    #[test]
    fn personal_registration_prefills_only_required_permissions_and_events() {
        let link = github_app_registration_url(
            "https://review.example.org",
            "Mamdouh OpenReview",
            None,
            false,
        )
        .unwrap();
        let url = Url::parse(&link).unwrap();
        assert_eq!(url.host_str(), Some("github.com"));
        assert_eq!(url.path(), "/settings/apps/new");

        let params: Vec<(String, String)> = url.query_pairs().into_owned().collect();
        for (key, value) in [
            ("name", "Mamdouh OpenReview"),
            ("url", "https://review.example.org/"),
            (
                "webhook_url",
                "https://review.example.org/api/webhooks/github",
            ),
            ("webhook_active", "true"),
            ("contents", "write"),
            ("issues", "write"),
            ("pull_requests", "write"),
            ("public", "false"),
        ] {
            assert!(params.contains(&(key.to_string(), value.to_string())));
        }
        let mut events: Vec<&str> = params
            .iter()
            .filter(|(k, _)| k == "events[]")
            .map(|(_, v)| v.as_str())
            .collect();
        events.sort_unstable();
        assert_eq!(
            events,
            [
                "installation",
                "installation_repositories",
                "issue_comment",
                "pull_request",
                "pull_request_review_comment"
            ]
        );
        assert!(params.iter().all(|(k, _)| k != "checks"));
        assert!(params.iter().all(|(k, _)| k != "webhook_secret"));
    }

    #[test]
    fn organization_registration_uses_safe_org_path_and_public_setting() {
        let link = github_app_registration_url(
            "https://pr.example.org:8443/",
            "My App",
            Some("my-org"),
            true,
        )
        .unwrap();
        let url = Url::parse(&link).unwrap();
        assert_eq!(url.path(), "/organizations/my-org/settings/apps/new");
        assert!(url.query_pairs().any(|(k, v)| k == "public" && v == "true"));
        assert!(url.query_pairs().any(|(k, v)| {
            k == "webhook_url" && v == "https://pr.example.org:8443/api/webhooks/github"
        }));
    }

    #[test]
    fn setup_link_rejects_credentials_insecure_or_ambiguous_hosts() {
        for base in [
            "http://review.example.org",
            "https://user:pass@review.example.org",
            "https://review.example.org/custom/path",
            "https://review.example.org?redirect=https://attacker.example",
            "https://review.example.org#section",
            "https://YOUR_DOMAIN",
            "file:///etc/passwd",
            "https://localhost:3000",
            "https://127.0.0.1:3000",
        ] {
            assert!(
                github_app_registration_url(base, "OpenReview", None, false).is_err(),
                "{base} should be rejected"
            );
        }
        for org in ["other/path", "-invalid", "invalid-", "bad org", "../admin"] {
            assert!(
                github_app_registration_url(
                    "https://review.example.org",
                    "MyApp",
                    Some(org),
                    false
                )
                .is_err()
            );
        }
        assert!(
            github_app_registration_url("https://review.example.org", "  ", None, false).is_err()
        );
    }
}

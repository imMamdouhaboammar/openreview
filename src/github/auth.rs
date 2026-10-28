use crate::error::{AppError, Result};
use crate::github::types::InstallationTokenResponse;
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    iat: u64,
    exp: u64,
    iss: String,
}

pub fn create_app_jwt(app_id: &str, private_key_pem: &str) -> Result<String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| AppError::Internal(e.to_string()))?
        .as_secs();

    // 60-second drift allowance, 9-minute validity (GitHub allows max 10m)
    let claims = Claims {
        iat: now.saturating_sub(60),
        exp: now + (9 * 60),
        iss: app_id.to_string(),
    };

    let header = Header::new(Algorithm::RS256);
    let encoding_key = EncodingKey::from_rsa_pem(private_key_pem.as_bytes())
        .map_err(|e| AppError::Auth(format!("Invalid RSA private key: {e}")))?;

    encode(&header, &claims, &encoding_key)
        .map_err(|e| AppError::Auth(format!("Failed to sign JWT: {e}")))
}

pub async fn get_installation_token(
    client: &reqwest::Client,
    app_id: &str,
    private_key_pem: &str,
    installation_id: u64,
) -> Result<String> {
    let jwt = create_app_jwt(app_id, private_key_pem)?;
    let url = format!("https://api.github.com/app/installations/{installation_id}/access_tokens");

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {jwt}"))
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "openreview-rs/0.1.0")
        .send()
        .await
        .map_err(|e| AppError::GitHubApi(format!("Failed to request installation token: {e}")))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::GitHubApi(format!(
            "Failed to obtain installation token (HTTP {status}): {body}"
        )));
    }

    let token_resp: InstallationTokenResponse = response
        .json()
        .await
        .map_err(|e| AppError::GitHubApi(format!("Failed to parse installation token: {e}")))?;

    Ok(token_resp.token)
}

use std::collections::HashMap;
use tokio::sync::RwLock;

#[derive(Default)]
pub struct TokenManager {
    tokens: RwLock<HashMap<u64, (String, u64)>>,
}

impl TokenManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn get_token(
        &self,
        client: &reqwest::Client,
        app_id: &str,
        private_key_pem: &str,
        installation_id: u64,
    ) -> Result<String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| AppError::Internal(e.to_string()))?
            .as_secs();

        // Check cache with 5-minute safety margin
        {
            let guard = self.tokens.read().await;
            if let Some((token, exp)) = guard.get(&installation_id)
                && *exp > now + 300
            {
                return Ok(token.clone());
            }
        }

        let fresh_token =
            get_installation_token(client, app_id, private_key_pem, installation_id).await?;
        let exp = now + 3600;

        {
            let mut guard = self.tokens.write().await;
            guard.insert(installation_id, (fresh_token.clone(), exp));
        }

        Ok(fresh_token)
    }

    pub async fn invalidate(&self, installation_id: u64) {
        let mut guard = self.tokens.write().await;
        guard.remove(&installation_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_claims_format() {
        // Validates Claims serialization
        let claims = Claims {
            iat: 1000,
            exp: 1540,
            iss: "12345".to_string(),
        };
        let serialized = serde_json::to_string(&claims).unwrap();
        assert!(serialized.contains("\"iss\":\"12345\""));
    }
}

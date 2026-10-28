use crate::error::{AppError, Result};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use sha2::digest::KeyInit;

type HmacSha256 = Hmac<Sha256>;

pub fn verify_signature(secret: &str, body: &[u8], signature_header: &str) -> Result<bool> {
    if secret.is_empty() {
        return Err(AppError::Config("Webhook secret is empty".to_string()));
    }

    let expected_prefix = "sha256=";
    if !signature_header.starts_with(expected_prefix) {
        return Ok(false);
    }

    let signature_hex = &signature_header[expected_prefix.len()..];
    let decoded_signature = match hex::decode(signature_hex) {
        Ok(sig) => sig,
        Err(_) => return Ok(false),
    };

    let mut mac = match HmacSha256::new_from_slice(secret.as_bytes()) {
        Ok(m) => m,
        Err(_) => return Err(AppError::Internal("Failed to initialize HMAC".to_string())),
    };

    mac.update(body);
    Ok(mac.verify_slice(&decoded_signature).is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_webhook_signature() {
        let secret = "test_webhook_secret_123";
        let body = b"{\"action\":\"created\",\"issue\":{\"number\":42}}";

        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        let digest = mac.finalize().into_bytes();
        let header = format!("sha256={}", hex::encode(digest));

        assert!(verify_signature(secret, body, &header).unwrap());
    }

    #[test]
    fn test_invalid_webhook_signature() {
        let secret = "test_webhook_secret_123";
        let body = b"{\"action\":\"created\"}";
        let invalid_header =
            "sha256=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

        assert!(!verify_signature(secret, body, invalid_header).unwrap());
    }

    #[test]
    fn test_malformed_header() {
        let secret = "test_webhook_secret_123";
        let body = b"test";

        assert!(!verify_signature(secret, body, "invalid-prefix").unwrap());
        assert!(!verify_signature(secret, body, "sha256=not-hex!").unwrap());
    }
}

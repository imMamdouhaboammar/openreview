use crate::error::{AppError, Result};
use reqwest::header::HeaderMap;
use std::time::Duration;

/// Every provider request, including reading streamed response bodies, must have
/// a finite upper bound. Values are evaluated when constructing each provider.
#[derive(Clone, Copy)]
pub struct HttpDeadlines {
    pub connect: Duration,
    pub request: Duration,
}

impl HttpDeadlines {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            connect: configured_duration("LLM_CONNECT_TIMEOUT_SECS", 10)?,
            request: configured_duration("LLM_REQUEST_TIMEOUT_SECS", 180)?,
        })
    }

    pub fn client(&self, headers: HeaderMap, prohibit_redirects: bool) -> Result<reqwest::Client> {
        let mut builder = reqwest::Client::builder()
            .default_headers(headers)
            .connect_timeout(self.connect)
            .timeout(self.request);
        if prohibit_redirects {
            builder = builder.redirect(reqwest::redirect::Policy::none());
        }
        builder
            .build()
            .map_err(|e| AppError::Internal(e.to_string()))
    }
}

pub fn configured_duration(name: &str, default_seconds: u64) -> Result<Duration> {
    let seconds = match std::env::var(name) {
        Ok(value) => value.parse::<u64>().map_err(|_| {
            AppError::Config(format!(
                "{name} must be a positive integer number of seconds"
            ))
        })?,
        Err(std::env::VarError::NotPresent) => default_seconds,
        Err(err) => return Err(AppError::Config(format!("Cannot read {name}: {err}"))),
    };
    validate_seconds(name, seconds)
}

fn validate_seconds(name: &str, seconds: u64) -> Result<Duration> {
    if !(1..=86_400).contains(&seconds) {
        return Err(AppError::Config(format!(
            "{name} must be between 1 and 86400 seconds"
        )));
    }
    Ok(Duration::from_secs(seconds))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeout_configuration_rejects_zero_and_overly_large_values() {
        for seconds in [0u64, 86_401] {
            assert!(validate_seconds("LLM_REQUEST_TIMEOUT_SECS", seconds).is_err());
        }
        assert_eq!(
            validate_seconds("LLM_REQUEST_TIMEOUT_SECS", 180).unwrap(),
            Duration::from_secs(180)
        );
    }
}

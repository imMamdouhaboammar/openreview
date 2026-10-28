use crate::error::{AppError, Result};
use std::collections::HashSet;
use std::env;

// This structure holds installation private keys, webhook HMAC material, and
// provider credentials; never derive Debug and accidentally log the secrets.
#[derive(Clone)]
pub struct Config {
    pub github_app_id: String,
    pub github_app_private_key: String,
    pub github_app_webhook_secret: String,
    pub github_app_installation_id: Option<u64>,
    pub allowed_installation_ids: HashSet<u64>,
    pub bot_username: String,

    pub llm_provider: String,
    pub model_name: String,
    pub anthropic_api_key: Option<String>,
    pub openai_api_key: Option<String>,
    pub openrouter_api_key: Option<String>,
    pub opencode_api_key: Option<String>,
    pub openai_base_url: Option<String>,
    pub gemini_api_key: Option<String>,
    pub ollama_base_url: Option<String>,

    pub sandbox_driver: String,
    pub job_api_token: Option<String>,
    pub max_concurrent_jobs: usize,
    pub sandbox_workspace_dir: String,
    pub host: String,
    pub port: u16,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let github_app_id = env::var("GITHUB_APP_ID").unwrap_or_default();
        let github_app_private_key = env::var("GITHUB_APP_PRIVATE_KEY")
            .map(|k| k.replace("\\n", "\n"))
            .unwrap_or_default();
        let github_app_webhook_secret = env::var("GITHUB_APP_WEBHOOK_SECRET").unwrap_or_default();

        let github_app_installation_id = env::var("GITHUB_APP_INSTALLATION_ID")
            .ok()
            .and_then(|id| id.parse::<u64>().ok());
        // A public GitHub App can be installed by any GitHub account. Signed
        // webhooks alone do not grant permission to spend this operator's LLM
        // budget or launch sandboxes. Default deny until explicitly approved.
        let mut allowed_installation_ids = HashSet::new();
        if let Ok(ids) = env::var("ALLOWED_INSTALLATION_IDS") {
            for item in ids.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                let id = item.parse::<u64>().map_err(|_| {
                    AppError::Config(format!(
                        "Invalid ALLOWED_INSTALLATION_IDS entry '{item}': expected numeric GitHub installation ID"
                    ))
                })?;
                if id == 0 {
                    return Err(AppError::Config(
                        "ALLOWED_INSTALLATION_IDS must contain nonzero installation IDs".into(),
                    ));
                }
                allowed_installation_ids.insert(id);
            }
        }

        let bot_username = env::var("BOT_USERNAME").unwrap_or_else(|_| "openreview".to_string());

        let llm_provider = env::var("LLM_PROVIDER")
            .unwrap_or_else(|_| "anthropic".to_string())
            .to_lowercase();

        let model_name = env::var("LLM_MODEL").unwrap_or_else(|_| match llm_provider.as_str() {
            "openai" => "gpt-4o".to_string(),
            "gemini" => "gemini-2.5-flash".to_string(),
            "ollama" => "deepseek-r1:latest".to_string(),
            // Aggregators have no single safe default model; require an exact
            // identifier from the customer's own account / subscription.
            "openrouter" | "opencode" | "opencode_go" => String::new(),
            _ => "claude-3-7-sonnet-20250219".to_string(),
        });

        let anthropic_api_key = env::var("ANTHROPIC_API_KEY").ok();
        let openai_api_key = env::var("OPENAI_API_KEY").ok();
        let openrouter_api_key = env::var("OPENROUTER_API_KEY").ok();
        let opencode_api_key = env::var("OPENCODE_API_KEY").ok();
        let openai_base_url = env::var("OPENAI_BASE_URL")
            .ok()
            .filter(|v| !v.trim().is_empty());
        let gemini_api_key = env::var("GEMINI_API_KEY").ok();
        let ollama_base_url = env::var("OLLAMA_BASE_URL")
            .ok()
            .filter(|v| !v.trim().is_empty());

        let sandbox_driver = env::var("SANDBOX_DRIVER")
            .unwrap_or_else(|_| "docker".to_string())
            .to_lowercase();
        let job_api_token = env::var("JOB_API_TOKEN")
            .ok()
            .filter(|v| !v.trim().is_empty());
        let max_concurrent_jobs = match env::var("MAX_CONCURRENT_JOBS") {
            Ok(value) => value.parse::<usize>().map_err(|_| {
                AppError::Config(
                    "MAX_CONCURRENT_JOBS must be a positive integer between 1 and 100".into(),
                )
            })?,
            Err(env::VarError::NotPresent) => 10,
            Err(err) => {
                return Err(AppError::Config(format!(
                    "MAX_CONCURRENT_JOBS cannot be read: {err}"
                )));
            }
        };
        let sandbox_workspace_dir = env::var("SANDBOX_WORKSPACE_DIR")
            .unwrap_or_else(|_| "/srv/openreview/workspaces".into());

        let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());

        let port = env::var("PORT")
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(3000);

        Ok(Self {
            github_app_id,
            github_app_private_key,
            github_app_webhook_secret,
            github_app_installation_id,
            allowed_installation_ids,
            bot_username,
            llm_provider,
            model_name,
            anthropic_api_key,
            openai_api_key,
            openrouter_api_key,
            opencode_api_key,
            openai_base_url,
            gemini_api_key,
            ollama_base_url,
            sandbox_driver,
            job_api_token,
            max_concurrent_jobs,
            sandbox_workspace_dir,
            host,
            port,
        })
    }

    pub fn validate_github_credentials(&self) -> Result<()> {
        if self.github_app_id.is_empty() {
            return Err(AppError::Config("GITHUB_APP_ID is missing".to_string()));
        }
        if self.github_app_private_key.is_empty() {
            return Err(AppError::Config(
                "GITHUB_APP_PRIVATE_KEY is missing".to_string(),
            ));
        }
        if self.github_app_webhook_secret.is_empty() {
            return Err(AppError::Config(
                "GITHUB_APP_WEBHOOK_SECRET is missing".to_string(),
            ));
        }
        Ok(())
    }

    pub fn validate_llm_credentials(&self) -> Result<()> {
        if self.model_name.trim().is_empty() {
            return Err(AppError::Config(format!(
                "LLM_MODEL must be set to an accessible model ID for provider '{}'",
                self.llm_provider
            )));
        }
        match self.llm_provider.as_str() {
            "anthropic" => {
                if self.anthropic_api_key.as_deref().unwrap_or("").is_empty() {
                    return Err(AppError::Config(
                        "ANTHROPIC_API_KEY is required for Anthropic provider".to_string(),
                    ));
                }
            }
            "openai" => {
                if self.openai_api_key.as_deref().unwrap_or("").is_empty() {
                    return Err(AppError::Config(
                        "OPENAI_API_KEY is required for OpenAI provider".to_string(),
                    ));
                }
            }
            "openrouter" => {
                if self
                    .openrouter_api_key
                    .as_deref()
                    .unwrap_or("")
                    .trim()
                    .is_empty()
                {
                    return Err(AppError::Config(
                        "OPENROUTER_API_KEY is required for OpenRouter provider".to_string(),
                    ));
                }
            }
            "opencode" | "opencode_go" => {
                if self
                    .opencode_api_key
                    .as_deref()
                    .unwrap_or("")
                    .trim()
                    .is_empty()
                {
                    return Err(AppError::Config(
                        "OPENCODE_API_KEY is required for OpenCode Console provider".to_string(),
                    ));
                }
            }
            "gemini" => {
                if self.gemini_api_key.as_deref().unwrap_or("").is_empty() {
                    return Err(AppError::Config(
                        "GEMINI_API_KEY is required for Gemini provider".to_string(),
                    ));
                }
            }
            "ollama" => {
                // Ollama local endpoint requires no API key
            }
            other => {
                return Err(AppError::Config(format!(
                    "Unsupported LLM provider '{other}'. Supported: anthropic, openai, openrouter, opencode, opencode_go, gemini, ollama"
                )));
            }
        }
        Ok(())
    }

    pub fn validate_for_serve(&self) -> Result<()> {
        self.validate_github_credentials()?;
        self.validate_llm_credentials()?;
        if self.sandbox_driver != "docker" {
            return Err(AppError::Config(
                "Webhook service requires SANDBOX_DRIVER=docker; local mode is for trusted CLI development only".to_string(),
            ));
        }
        if self.job_api_token.as_deref().unwrap_or("").len() < 32 {
            return Err(AppError::Config(
                "JOB_API_TOKEN must be configured with at least 32 characters to protect job metadata".to_string(),
            ));
        }
        if !(1..=100).contains(&self.max_concurrent_jobs) {
            return Err(AppError::Config(
                "MAX_CONCURRENT_JOBS must be between 1 and 100".into(),
            ));
        }
        Ok(())
    }
}

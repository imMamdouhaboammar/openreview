use crate::error::{AppError, Result};
use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub github_app_id: String,
    pub github_app_private_key: String,
    pub github_app_webhook_secret: String,
    pub github_app_installation_id: Option<u64>,
    pub bot_username: String,

    pub llm_provider: String,
    pub model_name: String,
    pub anthropic_api_key: Option<String>,
    pub openai_api_key: Option<String>,
    pub openai_base_url: Option<String>,
    pub gemini_api_key: Option<String>,
    pub ollama_base_url: Option<String>,

    pub sandbox_driver: String,
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

        let bot_username = env::var("BOT_USERNAME").unwrap_or_else(|_| "openreview".to_string());

        let llm_provider = env::var("LLM_PROVIDER")
            .unwrap_or_else(|_| "anthropic".to_string())
            .to_lowercase();

        let model_name = env::var("LLM_MODEL").unwrap_or_else(|_| match llm_provider.as_str() {
            "openai" => "gpt-4o".to_string(),
            "gemini" => "gemini-2.5-flash".to_string(),
            "ollama" => "deepseek-r1:latest".to_string(),
            _ => "claude-3-7-sonnet-20250219".to_string(),
        });

        let anthropic_api_key = env::var("ANTHROPIC_API_KEY").ok();
        let openai_api_key = env::var("OPENAI_API_KEY").ok();
        let openai_base_url = env::var("OPENAI_BASE_URL").ok();
        let gemini_api_key = env::var("GEMINI_API_KEY").ok();
        let ollama_base_url = env::var("OLLAMA_BASE_URL").ok();

        let sandbox_driver = env::var("SANDBOX_DRIVER")
            .unwrap_or_else(|_| "local".to_string())
            .to_lowercase();

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
            bot_username,
            llm_provider,
            model_name,
            anthropic_api_key,
            openai_api_key,
            openai_base_url,
            gemini_api_key,
            ollama_base_url,
            sandbox_driver,
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
}

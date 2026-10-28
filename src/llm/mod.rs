pub mod anthropic;
pub mod gemini;
pub mod openai;
pub mod traits;
pub mod types;

pub use anthropic::AnthropicProvider;
pub use gemini::GeminiProvider;
pub use openai::OpenAiProvider;
pub use traits::LlmProvider;
pub use types::*;

use crate::config::Config;
use crate::error::{AppError, Result};

pub fn create_llm_provider(config: &Config) -> Result<Box<dyn LlmProvider>> {
    match config.llm_provider.as_str() {
        "anthropic" => {
            let key = config.anthropic_api_key.clone().ok_or_else(|| {
                AppError::Config("ANTHROPIC_API_KEY is required for Anthropic provider".to_string())
            })?;
            Ok(Box::new(AnthropicProvider::new(
                key,
                config.model_name.clone(),
            )?))
        }
        "openai" => {
            let key = config.openai_api_key.clone().unwrap_or_default();
            Ok(Box::new(OpenAiProvider::new(
                key,
                config.model_name.clone(),
                config.openai_base_url.clone(),
            )?))
        }
        "openrouter" => {
            let key = config
                .openrouter_api_key
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| {
                    AppError::Config(
                        "OPENROUTER_API_KEY is required for OpenRouter provider".into(),
                    )
                })?;
            Ok(Box::new(OpenAiProvider::new(
                key.to_string(),
                config.model_name.clone(),
                Some("https://openrouter.ai/api".into()),
            )?))
        }
        "opencode" | "opencode_go" => {
            let key = config
                .opencode_api_key
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| {
                    AppError::Config(
                        "OPENCODE_API_KEY is required for OpenCode Console provider".into(),
                    )
                })?;
            let base = if config.llm_provider == "opencode_go" {
                "https://opencode.ai/inference/go/openai"
            } else {
                "https://opencode.ai/inference/openai"
            };
            Ok(Box::new(OpenAiProvider::new(
                key.to_string(),
                config.model_name.clone(),
                Some(base.into()),
            )?))
        }
        "gemini" => {
            let key = config.gemini_api_key.clone().ok_or_else(|| {
                AppError::Config("GEMINI_API_KEY is required for Gemini provider".to_string())
            })?;
            Ok(Box::new(GeminiProvider::new(
                key,
                config.model_name.clone(),
            )?))
        }
        "ollama" => {
            let base_url = config
                .ollama_base_url
                .clone()
                .unwrap_or_else(|| "http://localhost:11434".to_string());
            Ok(Box::new(OpenAiProvider::new(
                String::new(),
                config.model_name.clone(),
                Some(base_url),
            )?))
        }
        other => Err(AppError::Config(format!(
            "Unsupported LLM provider '{other}'. Supported: anthropic, openai, openrouter, opencode, opencode_go, gemini, ollama"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_for(provider: &str) -> Config {
        Config {
            github_app_id: String::new(),
            github_app_private_key: String::new(),
            github_app_webhook_secret: String::new(),
            github_app_installation_id: None,
            allowed_installation_ids: Default::default(),
            bot_username: "openreview".into(),
            llm_provider: provider.into(),
            model_name: "test-model".into(),
            anthropic_api_key: None,
            openai_api_key: None,
            openrouter_api_key: None,
            opencode_api_key: None,
            openai_base_url: None,
            gemini_api_key: None,
            ollama_base_url: None,
            sandbox_driver: "docker".into(),
            job_api_token: None,
            max_concurrent_jobs: 1,
            sandbox_workspace_dir: "/tmp/openreview-test".into(),
            host: "127.0.0.1".into(),
            port: 3000,
        }
    }

    #[test]
    fn model_identifier_must_be_selected_before_sending_keys_to_provider() {
        for provider in ["openrouter", "opencode", "opencode_go"] {
            let mut config = config_for(provider);
            config.model_name.clear();
            config.openrouter_api_key = Some("test-key".into());
            config.opencode_api_key = Some("test-key".into());
            let error = config.validate_llm_credentials().unwrap_err();
            assert!(error.to_string().contains("LLM_MODEL"), "{error}");
        }
    }

    #[test]
    fn openrouter_requires_its_own_key_instead_of_openai_credentials() {
        let mut config = config_for("openrouter");
        config.openai_api_key = Some("openai-key-not-authorized-for-openrouter".into());
        let message = match create_llm_provider(&config) {
            Err(AppError::Config(message)) => message,
            _ => panic!("OpenRouter without its own key must fail closed"),
        };
        assert!(message.contains("OPENROUTER_API_KEY"), "{message}");
    }

    #[test]
    fn opencode_requires_its_own_key_for_console_and_go() {
        for provider in ["opencode", "opencode_go"] {
            let mut config = config_for(provider);
            config.openai_api_key = Some("openai-key-not-authorized-for-opencode".into());
            let message = match create_llm_provider(&config) {
                Err(AppError::Config(message)) => message,
                _ => panic!("OpenCode without its own key must fail closed"),
            };
            assert!(message.contains("OPENCODE_API_KEY"), "{message}");
        }
    }

    #[test]
    fn openrouter_key_selects_the_ordinary_agent_llm_interface() {
        let mut config = config_for("openrouter");
        config.openrouter_api_key = Some("an-openrouter-test-key".into());
        let provider = create_llm_provider(&config).expect("OpenRouter key should be accepted");
        assert_eq!(provider.model_name(), "test-model");
    }

    #[test]
    fn opencode_console_and_go_use_the_ordinary_agent_llm_interface() {
        for selected in ["opencode", "opencode_go"] {
            let mut config = config_for(selected);
            config.opencode_api_key = Some("an-opencode-test-key".into());
            let provider = create_llm_provider(&config).expect("OpenCode key should be accepted");
            assert_eq!(provider.model_name(), "test-model");
        }
    }
}

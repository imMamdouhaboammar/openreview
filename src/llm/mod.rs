pub mod anthropic;
pub mod gemini;
pub mod openai;
pub mod responses;
pub mod traits;
pub mod types;

pub use anthropic::AnthropicProvider;
pub use gemini::GeminiProvider;
pub use openai::OpenAiProvider;
pub use responses::OpenAiResponsesProvider;
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
        "opencode_custom" => {
            config.validate_llm_credentials()?;
            let key = config.opencode_api_key.clone().expect("validated above");
            let base = config.validated_opencode_custom_url()?;
            let model = config.model_name.clone();
            let provider: Box<dyn LlmProvider> = match config.opencode_custom_protocol.as_deref() {
                Some("chat") => Box::new(
                    OpenAiProvider::new(key, model, Some(base.clone()))?
                        .with_chat_endpoint(format!("{base}/chat/completions"))?,
                ),
                Some("responses") => Box::new(OpenAiResponsesProvider::new(
                    key,
                    model,
                    format!("{base}/responses"),
                )?),
                Some("anthropic") => {
                    Box::new(AnthropicProvider::new(key, model)?.with_opencode_custom(base)?)
                }
                Some("gemini") => {
                    if !model.bytes().all(|character| {
                        character.is_ascii_alphanumeric() || matches!(character, b'-' | b'_' | b'.')
                    }) {
                        return Err(AppError::Config(
                            "Gemini model ID must contain only letters, digits, '.', '-', '_'"
                                .into(),
                        ));
                    }
                    Box::new(GeminiProvider::new(key, model)?.with_opencode_custom(base)?)
                }
                _ => unreachable!("custom protocol validated above"),
            };
            Ok(provider)
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
            "Unsupported LLM provider '{other}'. Supported: anthropic, openai, openrouter, opencode, opencode_go, opencode_custom, gemini, ollama"
        ))),
    }
}

#[cfg(test)]
mod custom_tests;

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
            opencode_custom_url: None,
            opencode_custom_protocol: None,
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

    #[test]
    fn custom_opencode_requires_a_connection_url_even_with_valid_key() {
        let mut config = config_for("opencode_custom");
        config.opencode_api_key = Some("test-opencode-key".into());
        let error = config.validate_llm_credentials().unwrap_err();
        assert!(
            error.to_string().contains("OPENCODE_CUSTOM_URL"),
            "custom connection must not accept a missing URL: {error}"
        );
    }

    #[test]
    fn custom_opencode_does_not_fall_back_to_another_provider_key() {
        let mut config = config_for("opencode_custom");
        config.openai_api_key = Some("unrelated-openai-key".into());
        let error = config.validate_llm_credentials().unwrap_err();
        assert!(
            error.to_string().contains("OPENCODE_API_KEY"),
            "custom OpenCode must use its own scoped token: {error}"
        );
    }

    #[test]
    fn custom_opencode_model_uses_its_own_connection() {
        for protocol in ["chat", "responses", "anthropic", "gemini"] {
            let mut config = config_for("opencode_custom");
            config.opencode_api_key = Some("a-scoped-opencode-key".into());
            config.opencode_custom_url =
                Some("https://opencode.ai/inference/custom/conn_ABC123".into());
            config.opencode_custom_protocol = Some(protocol.into());
            let provider = create_llm_provider(&config)
                .unwrap_or_else(|error| panic!("{protocol} adapter unavailable: {error}"));
            assert_eq!(provider.model_name(), "test-model");
        }
    }

    #[test]
    fn custom_opencode_rejects_credential_exfiltration_urls() {
        for unsafe_url in [
            "http://opencode.ai/inference/custom/conn_ABC",
            "https://evil.example/inference/custom/conn_ABC",
            "https://opencode.ai.evil.example/inference/custom/conn_ABC",
            "https://opencode.ai@evil.example/inference/custom/conn_ABC",
            "https://opencode.ai/inference/custom/conn_ABC/other",
            "https://opencode.ai/inference/custom/conn_ABC/",
            "https://opencode.ai/inference/custom/conn_ABC///",
            "https://opencode.ai/inference/custom/conn_ABC?redirect=https://evil.example",
            "https://opencode.ai/inference/custom/conn_ABC%2Fother",
        ] {
            let mut config = config_for("opencode_custom");
            config.opencode_api_key = Some("a-scoped-opencode-key".into());
            config.opencode_custom_protocol = Some("chat".into());
            config.opencode_custom_url = Some(unsafe_url.into());
            assert!(
                config.validate_llm_credentials().is_err(),
                "unsafe destination unexpectedly accepted: {unsafe_url}"
            );
        }
    }

    #[test]
    fn custom_opencode_requires_explicit_valid_protocol() {
        let mut config = config_for("opencode_custom");
        config.opencode_api_key = Some("a-scoped-opencode-key".into());
        config.opencode_custom_url = Some("https://opencode.ai/inference/custom/conn_ABC".into());
        for invalid in [None, Some("auto"), Some("file"), Some("gemini/../evil")] {
            config.opencode_custom_protocol = invalid.map(str::to_owned);
            let error = config.validate_llm_credentials().unwrap_err();
            assert!(error.to_string().contains("OPENCODE_CUSTOM_PROTOCOL"));
        }
    }
}

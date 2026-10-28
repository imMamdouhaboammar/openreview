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
            "Unsupported LLM provider '{other}'. Supported: anthropic, openai, gemini, ollama"
        ))),
    }
}

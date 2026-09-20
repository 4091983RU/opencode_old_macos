//! Конфигурация окружения для OpenAI-совместимых API.
//!
//! Приоритет: аргументы CLI > переменные окружения > значение по умолчанию.

use anyhow::{bail, Context, Result};

/// Настройки доступа к провайдеру.
#[derive(Debug, Clone)]
pub struct Config {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
}

impl Config {
    /// Собирает конфигурацию из аргументов CLI/окружения.
    pub fn load(base_url: Option<&str>, api_key: Option<&str>, model: Option<&str>) -> Result<Self> {
        if dotenvy::dotenv().is_err() {
            // .env не обязателен — работаем и без него.
        }

        let base_url = base_url
            .map(ToString::to_string)
            .or_else(|| std::env::var("OPENAI_BASE_URL").ok())
            .unwrap_or_else(|| "https://api.openai.com/v1".to_string());

        let base_url = base_url.trim_end_matches('/').to_string();

        let api_key = api_key
            .map(ToString::to_string)
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
            .context("не задан API-ключ: --api-key или OPENAI_API_KEY")?;

        let api_key = api_key.trim().to_string();
        if api_key.is_empty() {
            bail!("API-ключ пуст");
        }

        let model = model
            .map(ToString::to_string)
            .or_else(|| std::env::var("OPENAI_MODEL").ok())
            .unwrap_or_else(|| "gpt-4o-mini".to_string());

        Ok(Self {
            base_url,
            api_key,
            model,
        })
    }

    /// Адрес эндпоинта `/chat/completions`.
    pub fn chat_url(&self) -> String {
        format!("{}/chat/completions", self.base_url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_trailing_slash() {
        let mut cfg = Config {
            base_url: "https://example.test/v1/".to_string(),
            api_key: "k".to_string(),
            model: "m".to_string(),
        };
        cfg.base_url = cfg.base_url.trim_end_matches('/').to_string();
        assert_eq!(cfg.chat_url(), "https://example.test/v1/chat/completions");
    }
}
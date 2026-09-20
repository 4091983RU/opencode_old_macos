//! Провайдеры моделей и их протоколы.
//!
//! Поддерживаются:
//! - `openai` — любой OpenAI-совместимый `/chat/completions` (по умолчанию api.openai.com);
//! - `qwen` — DashScope (Qwen) в OpenAI-совместимом режиме;
//! - `zen` — шлюз OpenCode Zen (`https://opencode.ai/zen/v1`);
//! - `gigachat` — Сбер GigaChat (OAuth2 client credentials + `/chat/completions`);
//! - `yandexgpt` — Yandex GPT (нативная схема `/foundationModels/v1/completion`).

use anyhow::{bail, Result};

/// Провайдер модели.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    Openai,
    Qwen,
    Zen,
    Gigachat,
    Yandexgpt,
}

impl Provider {
    /// Допустимые имена провайдеров (для справки).
    pub const NAMES: &'static [&'static str] = &["openai", "qwen", "zen", "gigachat", "yandexgpt"];

    /// Разбирает имя провайдера (без учёта регистра).
    pub fn parse(name: &str) -> Result<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "openai" => Ok(Self::Openai),
            "qwen" => Ok(Self::Qwen),
            "zen" => Ok(Self::Zen),
            "gigachat" => Ok(Self::Gigachat),
            "yandexgpt" | "yandex" => Ok(Self::Yandexgpt),
            other => bail!(
                "неизвестный провайдер '{other}', ожидается одно из: {}",
                Self::NAMES.join(", ")
            ),
        }
    }

    /// Каноническое имя.
    pub fn name(self) -> &'static str {
        match self {
            Self::Openai => "openai",
            Self::Qwen => "qwen",
            Self::Zen => "zen",
            Self::Gigachat => "gigachat",
            Self::Yandexgpt => "yandexgpt",
        }
    }

    /// Верхнерегистровый префикс для переменных окружения.
    pub fn env_prefix(self) -> &'static str {
        match self {
            Self::Openai => "OPENAI",
            Self::Qwen => "DASHSCOPE",
            Self::Zen => "ZEN",
            Self::Gigachat => "GIGACHAT",
            Self::Yandexgpt => "YANDEX",
        }
    }

    /// Переменная окружения для основного секрета (у GigaChat — нет).
    pub fn key_env(self) -> Option<&'static str> {
        match self {
            Self::Openai => Some("OPENAI_API_KEY"),
            Self::Qwen => Some("DASHSCOPE_API_KEY"),
            Self::Zen => Some("OPENCODE_API_KEY"),
            Self::Gigachat => None,
            Self::Yandexgpt => Some("YANDEX_API_KEY"),
        }
    }

    /// Base URL по умолчанию.
    pub fn default_base_url(self) -> &'static str {
        match self {
            Self::Openai => "https://api.openai.com/v1",
            Self::Qwen => "https://dashscope.aliyuncs.com/compatible-mode/v1",
            Self::Zen => "https://opencode.ai/zen/v1",
            Self::Gigachat => "https://gigachat.devices.sberbank.ru/api/v1",
            Self::Yandexgpt => "https://llm.api.cloud.yandex.net/foundationModels",
        }
    }

    /// Модель по умолчанию.
    pub fn default_model(self) -> &'static str {
        match self {
            Self::Openai => "gpt-4o-mini",
            Self::Qwen => "qwen-plus",
            Self::Zen => "mimo-v2.5-free",
            Self::Gigachat => "GigaChat-Max",
            Self::Yandexgpt => "yandexgpt/latest",
        }
    }

    /// URL конечной точки «болтовни».
    pub fn chat_url(self, base_url: &str) -> String {
        let base = base_url.trim_end_matches('/');
        match self {
            Self::Yandexgpt => format!("{base}/v1/completion"),
            _ => format!("{base}/chat/completions"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_names_case_insensitive() {
        for (input, expect) in [
            ("openai", Provider::Openai),
            ("QWEN", Provider::Qwen),
            ("  zen  ", Provider::Zen),
            ("gigachat", Provider::Gigachat),
            ("yandexgpt", Provider::Yandexgpt),
            ("yandex", Provider::Yandexgpt),
        ] {
            assert_eq!(Provider::parse(input).unwrap(), expect, "для '{input}'");
        }
    }

    #[test]
    fn rejects_unknown() {
        assert!(Provider::parse("claude").is_err());
    }

    #[test]
    fn builds_chat_urls() {
        assert_eq!(
            Provider::Openai.chat_url("https://api.openai.com/v1"),
            "https://api.openai.com/v1/chat/completions"
        );
        assert_eq!(
            Provider::Zen.chat_url("https://opencode.ai/zen/v1"),
            "https://opencode.ai/zen/v1/chat/completions"
        );
        assert_eq!(
            Provider::Yandexgpt.chat_url("https://llm.api.cloud.yandex.net/foundationModels"),
            "https://llm.api.cloud.yandex.net/foundationModels/v1/completion"
        );
        assert_eq!(
            Provider::Openai.chat_url("https://api.openai.com/v1/"),
            "https://api.openai.com/v1/chat/completions"
        );
    }
}
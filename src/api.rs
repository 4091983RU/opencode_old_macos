//! HTTP-клиент к OpenAI-совместимому `/chat/completions`.
//!
//! Только не потоковый режим (v0.1): отправляем промпт, получаем полный ответ.
//! TLS — rustls, память под старые macOS не нужна.

use anyhow::{Context, Result};
use serde_json::json;

use crate::config::Config;

/// Ответ модели на промпт.
pub async fn chat(cfg: &Config, prompt: &str, max_tokens: u32) -> Result<String> {
    let client = reqwest::Client::builder()
        .build()
        .context("не удалось создать HTTP-клиент")?;

    let body = json!({
        "model": cfg.model,
        "messages": [{
            "role": "user",
            "content": prompt,
        }],
        "max_tokens": max_tokens,
        "stream": false,
    });

    let response = client
        .post(cfg.chat_url())
        .bearer_auth(&cfg.api_key)
        .json(&body)
        .send()
        .await
        .context("сетевой запрос не удался")?;

    let status = response.status();
    let bytes = response
        .bytes()
        .await
        .context("не удалось прочитать тело ответа")?;

    if !status.is_success() {
        let snippet = String::from_utf8_lossy(&bytes);
        anyhow::bail!("API вернул {status}: {}", truncate(&snippet, 512));
    }

    let value: serde_json::Value =
        serde_json::from_slice(&bytes).context("не удалось разобрать JSON ответа")?;

    let content = value
        .pointer("/choices/0/message/content")
        .and_then(|v| v.as_str())
        .context("в ответе нет /choices/0/message/content")?;

    tracing::debug!(
        usage = ?value.pointer("/usage"),
        "ответ получен"
    );

    Ok(content.trim().to_string())
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let head: String = s.chars().take(max).collect();
        format!("{head}…")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_shortens() {
        let s = "abcdefghij";
        assert_eq!(truncate(s, 5), "abcde…");
        assert_eq!(truncate(s, 50), s);
    }
}
//! HTTP-клиенты к провайдерам.
//!
//! Общий путь — OpenAI-совместимый `/chat/completions` (openai/qwen/zen/gigachat).
//! GigaChat сначала обменивает client credentials на токен (OAuth2), Yandex GPT
//! использует нативную схему `/foundationModels/v1/completion`.
//! Только не потоковый режим: полный ответ за один запрос.

use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::json;
use uuid::Uuid;

use crate::config::Config;
use crate::provider::Provider;

/// Единый вход: отправить промпт выбранному провайдеру, вернуть текст ответа.
pub async fn chat(cfg: &Config, prompt: &str) -> Result<String> {
    let client = reqwest::Client::builder()
        .build()
        .context("не удалось создать HTTP-клиент")?;

    match cfg.provider {
        Provider::Gigachat => {
            let token = gigachat_access_token(&client, cfg).await?;
            openai_compatible_chat(&client, cfg, &token, prompt).await
        }
        Provider::Yandexgpt => yandex_chat(&client, cfg, prompt).await,
        _ => {
            let key = cfg
                .api_key
                .as_deref()
                .context("не задан API-ключ провайдера")?;
            openai_compatible_chat(&client, cfg, key, prompt).await
        }
    }
}

/// OpenAI-совместимый `/chat/completions` с Bearer-авторизацией.
async fn openai_compatible_chat(
    client: &Client,
    cfg: &Config,
    key: &str,
    prompt: &str,
) -> Result<String> {
    let body = json!({
        "model": cfg.model,
        "messages": [{
            "role": "user",
            "content": prompt,
        }],
        "max_tokens": cfg.max_tokens,
        "stream": false,
    });

    let response = client
        .post(cfg.provider.chat_url(&cfg.base_url))
        .bearer_auth(key)
        .json(&body)
        .send()
        .await
        .context("сетевой запрос не удался")?;

    let (status, bytes) = read_response(response).await?;
    if !status.is_success() {
        return Err(api_error(status, &bytes));
    }

    let value: serde_json::Value =
        serde_json::from_slice(&bytes).context("не удалось разобрать JSON ответа")?;
    let content = value
        .pointer("/choices/0/message/content")
        .and_then(|v| v.as_str())
        .context("в ответе нет /choices/0/message/content")?;

    tracing::debug!(usage = ?value.pointer("/usage"), "ответ получен");
    Ok(content.trim().to_string())
}

/// GigaChat: обмен client credentials на токен доступа (OAuth2, scope=GIGACHAT_API_PERS).
async fn gigachat_access_token(client: &Client, cfg: &Config) -> Result<String> {
    let client_id = cfg
        .client_id
        .as_deref()
        .context("не задан GigaChat client_id")?;
    let client_secret = cfg
        .client_secret
        .as_deref()
        .context("не задан GigaChat client_secret")?;

    let rquid = Uuid::new_v4().to_string();

    let response = client
        .post("https://ngw.devices.sberbank.ru:9443/api/v2/oauth")
        .basic_auth(client_id, Some(client_secret))
        .header("RqUID", &rquid)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body("scope=GIGACHAT_API_PERS")
        .send()
        .await
        .context("сетевой запрос токена GigaChat не удался")?;

    let (status, bytes) = read_response(response).await?;
    if !status.is_success() {
        return Err(api_error(status, &bytes));
    }

    let value: serde_json::Value =
        serde_json::from_slice(&bytes).context("не удалось разобрать JSON токена")?;
    let token = value
        .get("access_token")
        .and_then(|v| v.as_str())
        .context("в ответе GigaChat нет access_token")?;

    tracing::debug!(
        expires_at = value.get("expires_at").and_then(|v| v.as_u64()),
        "токен GigaChat получен"
    );
    Ok(token.to_string())
}

/// Yandex GPT: нативная схема `/foundationModels/v1/completion`.
async fn yandex_chat(client: &Client, cfg: &Config, prompt: &str) -> Result<String> {
    let key = cfg.api_key.as_deref().context("не задан ключ Yandex GPT")?;
    let folder_id = cfg
        .folder_id
        .as_deref()
        .context("не задан folder_id Yandex")?;
    let model_uri = model_uri(folder_id, &cfg.model);

    let body = json!({
        "modelUri": model_uri,
        "completionOptions": {
            "stream": false,
            "temperature": 0.3,
            "maxTokens": cfg.max_tokens,
        },
        "messages": [{
            "role": "user",
            "text": prompt,
        }],
    });

    let mut request = client
        .post(cfg.provider.chat_url(&cfg.base_url))
        .json(&body);
    if cfg.iam_token {
        request = request.bearer_auth(key);
    } else {
        request = request.header("Authorization", format!("Api-Key {key}"));
    }

    let response = request
        .send()
        .await
        .context("сетевой запрос не удался")?;

    let (status, bytes) = read_response(response).await?;
    if !status.is_success() {
        return Err(api_error(status, &bytes));
    }

    let value: serde_json::Value =
        serde_json::from_slice(&bytes).context("не удалось разобрать JSON ответа")?;
    let content = value
        .pointer("/result/alternatives/0/message/text")
        .and_then(|v| v.as_str())
        .context("в ответе Yandex GPT нет /result/alternatives/0/message/text")?;

    tracing::debug!(
        usage = ?value.pointer("/result/usage"),
        "ответ получен"
    );
    Ok(content.trim().to_string())
}

/// modelUri: либо уже полный (`gpt://…` / `ds://…`), либо `gpt://<folder_id>/<model>`.
fn model_uri(folder_id: &str, model: &str) -> String {
    let model = model.trim();
    if model.starts_with("gpt://") || model.starts_with("ds://") {
        model.to_string()
    } else {
        format!("gpt://{folder_id}/{model}")
    }
}

/// Читает статус и тело ответа.
async fn read_response(response: reqwest::Response) -> Result<(reqwest::StatusCode, Vec<u8>)> {
    let status = response.status();
    let bytes = response
        .bytes()
        .await
        .context("не удалось прочитать тело ответа")?;
    Ok((status, bytes.to_vec()))
}

/// Ошибка с телом ответа API (обрезанным до разумного размера).
fn api_error(status: reqwest::StatusCode, bytes: &[u8]) -> anyhow::Error {
    let snippet = String::from_utf8_lossy(bytes);
    anyhow::anyhow!("API вернул {status}: {}", truncate(&snippet, 512))
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

    #[test]
    fn model_uri_builds_from_folder_and_model() {
        assert_eq!(
            model_uri("b1g", "yandexgpt/latest"),
            "gpt://b1g/yandexgpt/latest"
        );
    }

    #[test]
    fn model_uri_passes_full_uri_through() {
        assert_eq!(
            model_uri("b1g", "ds://b1g/full-model"),
            "ds://b1g/full-model"
        );
    }
}
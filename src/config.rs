//! Конфигурация доступа к провайдерам.
//!
//! Приоритет: аргументы CLI > переменные окружения > значения по умолчанию.
//! Провайдер выбирается `--provider` или переменной `PROVIDER`.
//!
//! Переменные окружения (префикс зависит от провайдера: OPENAI_, DASHSCOPE_,
//! ZEN_, GIGACHAT_, YANDEX_):
//! - `<PREFIX>_BASE_URL` — адрес API;
//! - `<PREFIX>_MODEL` — модель по умолчанию;
//! - ключ: OPENAI_API_KEY / DASHSCOPE_API_KEY / OPENCODE_API_KEY,
//!   для Yandex GPT — YANDEX_API_KEY или YANDEX_IAM_TOKEN (+ YANDEX_FOLDER_ID),
//!   для GigaChat — GIGACHAT_CLIENT_ID + GIGACHAT_CLIENT_SECRET.

use anyhow::{bail, Context, Result};

use crate::provider::Provider;

/// Настройки доступа к провайдеру.
#[derive(Debug, Clone)]
pub struct Config {
    pub provider: Provider,
    pub base_url: String,
    pub api_key: Option<String>,
    pub model: String,
    pub max_tokens: u32,
    /// GigaChat: идентификатор приложения (client_id).
    pub client_id: Option<String>,
    /// GigaChat: секрет приложения (client_secret).
    pub client_secret: Option<String>,
    /// Yandex GPT: идентификатор каталога (folder_id).
    pub folder_id: Option<String>,
    /// Yandex GPT: передавать ключ как IAM-токен (Bearer) вместо API-ключа.
    pub iam_token: bool,
    /// Не проверять сертификат сервера (для провайдеров с «нестандартными»
    /// корнями, например GigaChat). Включается флагом `--insecure`.
    pub insecure_tls: bool,
}

impl Config {
    #[allow(clippy::too_many_arguments)]
    /// Собирает конфигурацию из параметров CLI и окружения.
    pub fn load(
        provider: Option<&str>,
        base_url: Option<&str>,
        api_key: Option<&str>,
        model: Option<&str>,
        client_id: Option<&str>,
        client_secret: Option<&str>,
        folder_id: Option<&str>,
        iam_token: bool,
        max_tokens: u32,
        insecure_tls: bool,
    ) -> Result<Self> {
        if dotenvy::dotenv().is_err() {
            // .env не обязателен — работаем и без него.
        }
        Self::resolve_as(
            provider,
            base_url,
            api_key,
            model,
            client_id,
            client_secret,
            folder_id,
            iam_token,
            max_tokens,
            insecure_tls,
            |name| std::env::var(name),
        )
    }

    /// Чистое разрешение конфигурации; окружение подменяется `lookup` (для тестов).
    #[allow(clippy::too_many_arguments)]
    fn resolve_as(
        provider: Option<&str>,
        base_url: Option<&str>,
        api_key: Option<&str>,
        model: Option<&str>,
        client_id: Option<&str>,
        client_secret: Option<&str>,
        folder_id: Option<&str>,
        iam_token: bool,
        max_tokens: u32,
        insecure_tls: bool,
        lookup: impl Fn(&str) -> std::result::Result<String, std::env::VarError>,
    ) -> Result<Self> {
        let provider_name = match provider {
            Some(p) => p.to_string(),
            None => lookup("PROVIDER").unwrap_or_else(|_| "openai".into()),
        };
        let provider = Provider::parse(&provider_name)?;

        // Общие параметры.
        let base_url = base_url
            .map(ToString::to_string)
            .or_else(|| env(&lookup, &format!("{}_BASE_URL", provider.env_prefix())))
            .map(|s| s.trim_end_matches('/').to_string())
            .unwrap_or_else(|| provider.default_base_url().to_string());

        let model = model
            .map(ToString::to_string)
            .or_else(|| env(&lookup, &format!("{}_MODEL", provider.env_prefix())))
            .unwrap_or_else(|| provider.default_model().to_string());

        let cli_key = api_key.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());

        // Секреты, специфичные для провайдера.
        let (api_key, iam_token) = match provider {
            Provider::Gigachat => (cli_key.or_else(|| env(&lookup, "GIGACHAT_API_KEY")), false),
            Provider::Openai | Provider::Qwen | Provider::Zen => {
                let key = cli_key
                    .or_else(|| provider.key_env().and_then(|n| env(&lookup, n)))
                    .context(key_hint(provider))?;
                if key.is_empty() {
                    bail!("API-ключ пуст");
                }
                (Some(key), false)
            }
            Provider::Yandexgpt => {
                let dk = cli_key.or_else(|| env(&lookup, "YANDEX_API_KEY"));
                let iam = env(&lookup, "YANDEX_IAM_TOKEN");
                let is_iam = iam_token || iam.is_some();
                let key = dk
                    .or(iam)
                    .context("для yandexgpt задайте ключ: --api-key, YANDEX_API_KEY или YANDEX_IAM_TOKEN")?;
                if key.is_empty() {
                    bail!("API-ключ пуст");
                }
                (Some(key), is_iam)
            }
        };

        let client_id = client_id
            .map(ToString::to_string)
            .or_else(|| env(&lookup, "GIGACHAT_CLIENT_ID"))
            .filter(|_| provider == Provider::Gigachat);
        let client_secret = client_secret
            .map(ToString::to_string)
            .or_else(|| env(&lookup, "GIGACHAT_CLIENT_SECRET"))
            .filter(|_| provider == Provider::Gigachat);
        if provider == Provider::Gigachat
            && api_key.is_none()
            && (client_id.is_none() || client_secret.is_none())
        {
            bail!(
                "для gigachat нужен ключ авторизации --api-key или пара --client-id/--client-secret \
                 (либо GIGACHAT_CLIENT_ID/GIGACHAT_CLIENT_SECRET)"
            );
        }

        let folder_id = folder_id
            .map(ToString::to_string)
            .or_else(|| env(&lookup, "YANDEX_FOLDER_ID"))
            .filter(|_| provider == Provider::Yandexgpt);
        if provider == Provider::Yandexgpt && folder_id.is_none() {
            bail!("для yandexgpt нужен folder_id: --folder-id или YANDEX_FOLDER_ID");
        }

        // Флаг --insecure можно включать и через окружение (удобно для .env).
        let insecure_tls = insecure_tls || flag_env(&lookup, "OPCODE_INSECURE");

        Ok(Self {
            provider,
            base_url,
            api_key,
            model,
            max_tokens,
            client_id,
            client_secret,
            folder_id,
            iam_token,
            insecure_tls,
        })
    }
}

/// Читает переменную окружения, отбрасывая пустые значения.
fn env(
    lookup: &impl Fn(&str) -> std::result::Result<String, std::env::VarError>,
    name: &str,
) -> Option<String> {
    lookup(name)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// Читает булевый флаг из окружения: 1/true/yes/on (без учёта регистра).
fn flag_env(
    lookup: &impl Fn(&str) -> std::result::Result<String, std::env::VarError>,
    name: &str,
) -> bool {
    env(lookup, name)
        .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

/// Подсказка, где взять ключ.
fn key_hint(provider: Provider) -> String {
    match provider.key_env() {
        Some(env) => format!("для провайдера {} нужен ключ: --api-key или {env}", provider.name()),
        None => format!("для провайдера {} этот секрет не нужен", provider.name()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_env(_name: &str) -> std::result::Result<String, std::env::VarError> {
        Err(std::env::VarError::NotPresent)
    }

    fn with<'a>(
        pairs: &'a [(&'a str, &'a str)],
    ) -> impl Fn(&str) -> std::result::Result<String, std::env::VarError> + 'a {
        move |name: &str| {
            pairs
                .iter()
                .find(|(k, _)| *k == name)
                .map(|(_, v)| v.to_string())
                .ok_or(std::env::VarError::NotPresent)
        }
    }

    #[test]
    fn openai_defaults() {
        let cfg = Config::resolve_as(
            None,
            None,
            Some("k"),
            None,
            None,
            None,
            None,
            false,
            2048,
            false,
            no_env,
        )
        .unwrap();
        assert_eq!(cfg.provider, Provider::Openai);
        assert_eq!(cfg.base_url, "https://api.openai.com/v1");
        assert_eq!(cfg.model, "gpt-4o-mini");
        assert_eq!(cfg.api_key.as_deref(), Some("k"));
    }

    #[test]
    fn openai_requires_key() {
        assert!(Config::resolve_as(None, None, None, None, None, None, None, false, 2048, false, no_env).is_err());
        assert!(Config::resolve_as(Some("openai"), None, None, None, None, None, None, false, 2048, false, no_env)
            .is_err());
    }

    #[test]
    fn qwen_reads_env_and_model() {
        let env = with(&[("DASHSCOPE_API_KEY", "k"), ("DASHSCOPE_MODEL", "qwen-max")]);
        let cfg = Config::resolve_as(Some("qwen"), None, None, None, None, None, None, false, 2048, false, env).unwrap();
        assert_eq!(cfg.api_key.as_deref(), Some("k"));
        assert_eq!(cfg.model, "qwen-max");
    }

    #[test]
    fn zen_uses_opencode_key_env() {
        let env = with(&[("OPENCODE_API_KEY", "sk-zen")]);
        let cfg = Config::resolve_as(Some("zen"), None, None, None, None, None, None, false, 2048, false, env).unwrap();
        assert_eq!(cfg.api_key.as_deref(), Some("sk-zen"));
        assert_eq!(cfg.base_url, "https://opencode.ai/zen/v1");
    }

    #[test]
    fn gigachat_needs_client_credentials() {
        assert!(Config::resolve_as(Some("gigachat"), None, None, None, None, None, None, false, 2048, false, no_env)
            .is_err());

        let env = with(&[
            ("GIGACHAT_CLIENT_ID", "id"),
            ("GIGACHAT_CLIENT_SECRET", "secret"),
        ]);
        let cfg = Config::resolve_as(Some("gigachat"), None, None, None, None, None, None, false, 2048, false, env).unwrap();
        assert_eq!(cfg.client_id.as_deref(), Some("id"));
        assert_eq!(cfg.client_secret.as_deref(), Some("secret"));
        assert!(cfg.api_key.is_none());

        // Готовый ключ авторизации тоже можно взять из окружения.
        let env = with(&[("GIGACHAT_API_KEY", "aWQ6c2VjcmV0")]);
        let cfg = Config::resolve_as(Some("gigachat"), None, None, None, None, None, None, false, 2048, false, env).unwrap();
        assert_eq!(cfg.api_key.as_deref(), Some("aWQ6c2VjcmV0"));
        assert!(cfg.client_id.is_none());
    }

    #[test]
    fn provider_from_env_overrides_default() {
        let env = with(&[
            ("PROVIDER", "gigachat"),
            ("GIGACHAT_API_KEY", "aWQ6c2VjcmV0"),
        ]);
        let cfg = Config::resolve_as(None, None, None, None, None, None, None, false, 2048, false, env)
            .unwrap();
        assert_eq!(cfg.provider, Provider::Gigachat);
        assert_eq!(cfg.api_key.as_deref(), Some("aWQ6c2VjcmV0"));
    }

    #[test]
    fn insecure_flag_from_env() {
        let env = with(&[("OPCODE_INSECURE", "1")]);
        let cfg = Config::resolve_as(
            Some("openai"),
            None,
            Some("k"),
            None,
            None,
            None,
            None,
            false,
            2048,
            false,
            env,
        )
        .unwrap();
        assert!(cfg.insecure_tls);

        let cfg = Config::resolve_as(
            Some("openai"),
            None,
            Some("k"),
            None,
            None,
            None,
            None,
            false,
            2048,
            false,
            no_env,
        )
        .unwrap();
        assert!(!cfg.insecure_tls);
    }

    #[test]
    fn yandex_requires_folder_and_accepts_iam() {
        assert!(Config::resolve_as(Some("yandexgpt"), None, None, None, None, None, None, false, 2048, false, no_env)
            .is_err());

        let env = with(&[
            ("YANDEX_API_KEY", "av7"),
            ("YANDEX_FOLDER_ID", "b1g"),
        ]);
        let cfg = Config::resolve_as(Some("yandexgpt"), None, None, None, None, None, Some("b1g"), false, 2048, false, env).unwrap();
        assert_eq!(cfg.api_key.as_deref(), Some("av7"));
        assert!(!cfg.iam_token);
        assert_eq!(cfg.folder_id.as_deref(), Some("b1g"));

        let env = with(&[("YANDEX_IAM_TOKEN", "t0ken"), ("YANDEX_FOLDER_ID", "b1g")]);
        let cfg = Config::resolve_as(Some("yandexgpt"), None, None, None, None, None, None, false, 2048, false, env).unwrap();
        assert_eq!(cfg.api_key.as_deref(), Some("t0ken"));
        assert!(cfg.iam_token);
    }

    #[test]
    fn base_url_from_cli_overrides_env() {
        let env = with(&[("OPENAI_BASE_URL", "https://env.example/v1")]);
        let cfg = Config::resolve_as(
            None,
            Some("https://cli.example/v1"),
            Some("k"),
            None,
            None,
            None,
            None,
            false,
            2048,
            false,
            env,
        )
        .unwrap();
        assert_eq!(cfg.base_url, "https://cli.example/v1");
    }
}
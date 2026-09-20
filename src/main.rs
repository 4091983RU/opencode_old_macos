//! opencode_old_macos — тонкий AI-агент (CLI).
//!
//! Цель: работать на старых macOS (10.13 и раньше), поэтому:
//! - единственная рантайм-зависимость — наша библиотека (нет Node/Bun/Electron);
//! - TLS реализован через rustls (не зависит от системного OpenSSL);
//! - TUI — обычный ANSI/stdio (пока простая печать в stdout).
//!
//! Провайдеры: openai, qwen, zen, gigachat, yandexgpt. Не потоковый ответ.

mod api;
mod config;
mod provider;

use anyhow::{Context, Result};
use clap::Parser;

use crate::config::Config;

/// Аргументы командной строки.
#[derive(Parser, Debug)]
#[command(
    name = "opencode_old_macos",
    version,
    about = "Тонкий AI-агент для старых macOS (несколько провайдеров: openai, qwen, zen, gigachat, yandexgpt)"
)]
struct Cli {
    /// Промпт (запрос к модели)
    prompt: String,

    /// Провайдер: openai, qwen, zen, gigachat, yandexgpt
    #[arg(long, default_value = "openai")]
    provider: String,

    /// Base URL провайдера (переопределяет стандартный адрес)
    #[arg(long)]
    base_url: Option<String>,

    /// API-ключ (у GigaChat вместо него используются --client-id/--client-secret)
    #[arg(long)]
    api_key: Option<String>,

    /// Имя модели (у Yandex GPT можно передать полный modelUri: gpt://<folder>/<model>)
    #[arg(long)]
    model: Option<String>,

    /// Максимум токенов в ответе
    #[arg(long, default_value_t = 2048)]
    max_tokens: u32,

    /// GigaChat: идентификатор приложения
    #[arg(long)]
    client_id: Option<String>,

    /// GigaChat: секрет приложения
    #[arg(long)]
    client_secret: Option<String>,

    /// Yandex GPT: идентификатор каталога (folder_id)
    #[arg(long)]
    folder_id: Option<String>,

    /// Yandex GPT: ключ — IAM-токен (Bearer) вместо API-ключа
    #[arg(long)]
    iam_token: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let cli = Cli::parse();
    let cfg = Config::load(
        Some(&cli.provider),
        cli.base_url.as_deref(),
        cli.api_key.as_deref(),
        cli.model.as_deref(),
        cli.client_id.as_deref(),
        cli.client_secret.as_deref(),
        cli.folder_id.as_deref(),
        cli.iam_token,
        cli.max_tokens,
    )
    .context("не удалось собрать конфигурацию")?;

    tracing::debug!(
        provider = %cfg.provider.name(),
        base_url = %cfg.base_url,
        model = %cfg.model,
        "выполняется запрос"
    );

    let text = api::chat(&cfg, &cli.prompt)
        .await
        .context("запрос к модели не удался")?;

    println!("{text}");
    Ok(())
}

fn init_tracing() {
    use tracing_subscriber::EnvFilter;
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,opencode_old_macos=info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();
}
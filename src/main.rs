//! opencode_old_macos — тонкий AI-агент (CLI).
//!
//! Цель: работать на старых macOS (10.13 и раньше), поэтому:
//! - единственный рантайм-зависимость — наша библиотека (нет Node/Bun/Electron);
//! - TLS реализован через rustls (не зависит от системного OpenSSL);
//! - TUI — обычный ANSI/stdio (пока простая печать в stdout).
//!
//! v0.1: один OpenAI-совместимый вход, не потоковый ответ.

mod api;
mod config;

use anyhow::{Context, Result};
use clap::Parser;

use crate::config::Config;

/// Аргументы командной строки.
#[derive(Parser, Debug)]
#[command(
    name = "opencode_old_macos",
    version,
    about = "Тонкий AI-агент для старых macOS (OpenAI-совместимые API)"
)]
struct Cli {
    /// Промпт (запрос к модели)
    prompt: String,

    /// OpenAI-совместимый base URL (переопределяет OPENAI_BASE_URL)
    #[arg(long)]
    base_url: Option<String>,

    /// API-ключ (переопределяет OPENAI_API_KEY)
    #[arg(long)]
    api_key: Option<String>,

    /// Имя модели (переопределяет OPENAI_MODEL)
    #[arg(long)]
    model: Option<String>,

    /// Максимум токенов в ответе
    #[arg(long, default_value_t = 2048)]
    max_tokens: u32,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let cli = Cli::parse();
    let cfg = Config::load(
        cli.base_url.as_deref(),
        cli.api_key.as_deref(),
        cli.model.as_deref(),
    )
    .context("не удалось собрать конфигурацию")?;

    tracing::debug!(base_url = %cfg.base_url, model = %cfg.model, "выполняется запрос");

    let text = api::chat(&cfg, &cli.prompt, cli.max_tokens)
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
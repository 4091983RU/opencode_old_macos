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
use std::io::{BufRead, IsTerminal, Write};

use crate::config::Config;

/// Аргументы командной строки.
#[derive(Parser, Debug)]
#[command(
    name = "opencode_old_macos",
    version,
    about = "Тонкий AI-агент для старых macOS (несколько провайдеров: openai, qwen, zen, gigachat, yandexgpt)"
)]
struct Cli {
    /// Промпт (запрос к модели). Если не задан и stdin — терминал,
    /// запускается интерактивный режим диалога; если stdin — не терминал,
    /// весь ввод читается как один промпт.
    prompt: Option<String>,

    /// Провайдер: openai, qwen, zen, gigachat, yandexgpt.
    /// Если не задан, берётся из переменной PROVIDER (в т.ч. из .env), иначе — openai.
    #[arg(long)]
    provider: Option<String>,

    /// Base URL провайдера (переопределяет стандартный адрес)
    #[arg(long)]
    base_url: Option<String>,

    /// API-ключ (у GigaChat это готовый ключ авторизации: base64 от client_id:client_secret,
    /// вместо --client-id/--client-secret; у остальных провайдеров — их ключ)
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

    /// Не проверять TLS-сертификат (нужно для GigaChat: его корневые
    /// сертификаты отсутствуют в стандартных хранилищах) — только для
    /// доверенной сети!
    #[arg(long)]
    insecure: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let cli = Cli::parse();
    let cfg = Config::load(
        cli.provider.as_deref(),
        cli.base_url.as_deref(),
        cli.api_key.as_deref(),
        cli.model.as_deref(),
        cli.client_id.as_deref(),
        cli.client_secret.as_deref(),
        cli.folder_id.as_deref(),
        cli.iam_token,
        cli.max_tokens,
        cli.insecure,
    )
    .context("не удалось собрать конфигурацию")?;

    tracing::debug!(
        provider = %cfg.provider.name(),
        base_url = %cfg.base_url,
        model = %cfg.model,
        "выполняется запрос"
    );

    match cli.prompt {
        Some(prompt) => {
            let text = api::chat(&cfg, &prompt)
                .await
                .context("запрос к модели не удался")?;
            println!("{text}");
        }
        None if std::io::stdin().is_terminal() => {
            interactive(&cfg).await?;
        }
        None => {
            // stdin — не терминал: читаем весь ввод как один промпт.
            let mut lines = String::new();
            for line in std::io::stdin().lock().lines() {
                lines.push_str(&line.map_err(anyhow::Error::from)?);
                lines.push('\n');
            }
            let text = api::chat(&cfg, lines.trim())
                .await
                .context("запрос к модели не удался")?;
            println!("{text}");
        }
    }

    Ok(())
}

/// Интерактивный диалог: читает вопросы из stdin, печатает ответы модели.
async fn interactive(cfg: &Config) -> Result<()> {
    println!(
        "Интерактивный режим ({} / {}). Вопрос — ответ; пустая строка — выход.",
        cfg.provider.name(),
        cfg.base_url
    );
    let stdin = std::io::stdin();
    let mut out = std::io::stdout();
    loop {
        out.write_all(b"> ").ok();
        out.flush().ok();
        let mut line = String::new();
        if stdin.lock().read_line(&mut line).ok().filter(|n| *n > 0).is_none() {
            break; // Ctrl+D
        }
        let prompt = line.trim();
        if prompt.is_empty() {
            break;
        }
        match api::chat(cfg, prompt).await {
            Ok(text) => println!("\n{}\n", text),
            Err(e) => eprintln!("ошибка: {e:#}"),
        }
    }
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
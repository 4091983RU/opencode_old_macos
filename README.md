# opencode_old_macos

Тонкий AI-агент (CLI) для работы на **старых macOS** (High Sierra 10.13 и ранее, Intel).
OpenAI-совместимые API. Единственная рантайм-зависимость — собственный Rust-бинарник:
никаких Bun/Node/Electron, TLS — собственный (rustls).

## Зачем это

Современные AI-агенты (opencode и др.) требуют macOS 12–13+, потому что их рантаймы
(Bun, Electron, новый Node) собраны под новые ОС. На MacBookPro8,3 (2011, потолок
официально — 10.13) они не запускаются. Этот проект — агент, который работает там, где
всё остальное — нет.

Минимальная поддерживаемая macOS задаётся на этапе компиляции
(`MACOSX_DEPLOYMENT_TARGET` в `.cargo/config.toml`) и **проверяется в CI** по
маш-бинарнику (`script/check-minos.sh`).

## Быстрый старт

```bash
export OPENAI_BASE_URL="https://api.openai.com/v1"   # или любой совместимый: vLLM, Ollama, гейтвей
export OPENAI_API_KEY="sk-..."
export OPENAI_MODEL="gpt-4o-mini"

cargo run -- "объясни, почему список Temu нельзя суммировать"
```

Все три параметра можно передать аргументами: `--base-url`, `--api-key`, `--model`.

## Конфигурация

| Переменная            | Аргумент CLI    | По умолчанию                              |
| --------------------- | --------------- | ----------------------------------------- |
| `OPENAI_BASE_URL`     | `--base-url`    | `https://api.openai.com/v1`               |
| `OPENAI_API_KEY`      | `--api-key`     | нет (обязателен)                          |
| `OPENAI_MODEL`        | `--model`       | `gpt-4o-mini`                             |

Поддерживается `.env` через `dotenvy`.

## Сборка для старых macOS

```bash
rustup target add x86_64-apple-darwin
MACOSX_DEPLOYMENT_TARGET=10.9 cargo build --release --target x86_64-apple-darwin
# проверить минимальную версию бинарника:
bash script/check-minos.sh target/x86_64-apple-darwin/release/opencode_old_macos
```

CI (GitHub Actions) собирает `darwin-x64`, проверяет minOS и публикует артефакт.
Подробнее — [ARCHITECTURE.md](ARCHITECTURE.md).

## Роадмап (MVP на ~2 месяца)

- **v0.1 / нед. 1–2** — CLI: промпт → ответ, конфиг, логирование через `tracing`.
  (текущее состояние)
- **нед. 3** — потоковый ответ (SSE), отмена по `Ctrl+C`, retry.
- **нед. 4** — история/сессии в локальном JSON (`~/.config/opencode_old_macos/`).
- **нед. 5–6** — инструменты: чтение/запись файлов, применение патчей, require-подтверждение.
- **нед. 7–8** — простой TUI на `ratatui` (ANSI/stdio, работает на 10.13).
- **дальше** — энтерпрайз-слой: доменная авторизация, политики, лимиты, аудит.

## Лицензия

MIT
# opencode_old_macos

Тонкий AI-агент (CLI) для работы на **старых macOS** (High Sierra 10.13 и ранее, Intel).
Поддерживаются провайдеры: OpenAI-совместимые (openai/qwen/zen), GigaChat, Yandex GPT.
Единственная рантайм-зависимость — собственный Rust-бинарник:
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
export OPENAI_API_KEY="sk-..."   # свой ключ OpenAI

cargo run -- "объясни, почему список Temu нельзя суммировать"
# или с провайдером:
cargo run --provider qwen -- "привет"
```

## Провайдеры

Провайдер выбирается `--provider` / `PROVIDER`. Переменные окружения зависят от
провайдера (`<PREFIX>_BASE_URL`, `<PREFIX>_MODEL`, ключ — см. таблицу).

| Провайдер    | Ключ (env)                                    | Base URL по умолчанию                | Модель по умолчанию |
| ------------ | --------------------------------------------- | ------------------------------------ | ------------------- |
| `openai`     | `OPENAI_API_KEY`                              | `https://api.openai.com/v1`          | `gpt-4o-mini`       |
| `qwen`       | `DASHSCOPE_API_KEY`                           | `https://dashscope.aliyuncs.com/compatible-mode/v1` | `qwen-plus` |
| `zen`        | `OPENCODE_API_KEY`                            | `https://opencode.ai/zen/v1`         | `mimo-v2.5-free`    |
| `gigachat`   | `GIGACHAT_CLIENT_ID` + `GIGACHAT_CLIENT_SECRET` | `https://gigachat.devices.sberbank.ru/api/v1` | `GigaChat-Max` |
| `yandexgpt`  | `YANDEX_API_KEY` (или `YANDEX_IAM_TOKEN`) + `YANDEX_FOLDER_ID` | `https://llm.api.cloud.yandex.net/foundationModels` | `yandexgpt/latest` |

Все параметры можно передать аргументами: `--base-url`, `--api-key`, `--model`,
а также `--client-id`/`--client-secret` (GigaChat), `--folder-id`/`--iam-token`
(Yandex GPT). Yandex GPT принимает полный modelUri: `--model "gpt://<folder>/<model>"`.

OpenAI-совместимый (`openai`) работает с любым совместимым API: vLLM, Ollama, гейтвей.

## Конфигурация

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

## Лицензия

MIT
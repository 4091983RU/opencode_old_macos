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

## Установка и обновление (рекомендуется)

Бинарник для мака сестры собирает **GitHub Actions** (нужен macOS-раннер), а на
gitverse таких раннеров нет. Поэтому канонический репозиторий — gitverse, а на
GitHub он автоматически зеркалируется, где и срабатывает CI.

1. Пуш в gitverse (ветка `master`). gitverse-CI (`sync-to-github` в
   `.gitverse/workflows/sync-to-github.yml`) синхронизирует коммит на GitHub-зеркало.
   Нужны секреты `GITHUBTOKEN` и `GITHUBMIRRORREPO` (`4091983RU/opencode_old_macos`)
   в Настройки → Секреты и переменные гитверса (подчёркивания gitverse не
   принимает), а в настройках CI/CD репозитория выбрано «Использовать конфигурацию
   из .gitverse/workflows».
2. GitHub Actions собирает артефакт `opencode_old_macos-darwin-x64` и проверяет
   minOS ≤ 10.13 (`script/check-minos.sh`).
3. Во вкладке **Actions** последнего прогона скачай артефакт
   `opencode_old_macos-darwin-x64`.
4. На маке сестры (Rust не нужен):

   ```bash
   unzip opencode_old_macos-darwin-x64.zip
   chmod +x opencode_old_macos            # артефакт может терять бит исполнения
   xattr -dr com.apple.quarantine opencode_old_macos   # если Gatekeeper ругается
   export OPENAI_API_KEY="sk-..."         # или другой провайдер, см. ниже
   ./opencode_old_macos "привет"
   ```

**Обновление** — повтор той же процедуры: пуш в gitverse → свежий артефакт в
GitHub Actions → заменил бинарник на маке. Это единый скомпилированный бинарник,
поэтому каждая новая версия = пересборка (её делает CI, а не мак).

## Сборка из исходников (по необходимости)

Обычно исходники собирать не нужно — берите артефакт из CI (см. выше).

Локальная сборка требует macOS и Xcode Command Line Tools (rustc на x86_64
поддерживает macOS 10.12+):

```bash
# на Intel-маке x86_64-apple-darwin — это родной host-таргет, `target add` не нужен;
# при кросс-сборке с другой машины выполните:
rustup target add x86_64-apple-darwin

MACOSX_DEPLOYMENT_TARGET=10.9 cargo build --release --target x86_64-apple-darwin
# minOS задан в .cargo/config.toml; переменная нужна, только если его переопределить;
# проверить минимальную версию бинарника:
bash script/check-minos.sh target/x86_64-apple-darwin/release/opencode_old_macos
```

На самом MacBookPro8,3 такая сборка возможна, но медленная (компиляция
`ring`/`rustls`/`tokio` на железе 2011 года) — предпочтительнее артефакт из CI.

CI (GitHub Actions на GitHub-зеркале) собирает `darwin-x64`, проверяет minOS и
публикует артефакт. Подробнее — [ARCHITECTURE.md](ARCHITECTURE.md).

## Лицензия

MIT
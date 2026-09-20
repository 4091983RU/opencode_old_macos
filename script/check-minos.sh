#!/usr/bin/env bash
# Проверяет, что задекларированная в Mach-O бинарника минимальная macOS
# не новее переданного порога (по умолчанию 10.13).
#
# Использование:
#   bash script/check-minos.sh <path-to-binary> [major] [minor]
set -euo pipefail

BIN="${1:?укажите путь к бинарнику}"
MAX_MAJOR="${2:-10}"
MAX_MINOR="${3:-13}"

if [[ ! -x "$BIN" ]]; then
  echo "binary not found: $BIN" >&2
  exit 1
fi

# otool печатает minimum OS двумя способами — в зависимости от того, какой
# load command эмитировал линкер:
#   - современный LC_BUILD_VERSION      -> поле "minos  10.9"
#   - устаревший LC_VERSION_MIN_MACOSX  -> поле "version 10.9"
# Ищем значение внутри соответствующего блока load command.
minos="$(otool -l "$BIN" | awk '
  $1 == "cmd" { want = ($2 ~ /LC_BUILD_VERSION|LC_VERSION_MIN_MACOSX/); next }
  want && ($1 == "minos" || $1 == "version") { print $2; exit }
')"
if [[ -z "$minos" ]]; then
  echo "minos/version not found in $BIN — не Mach-O?" >&2
  otool -l "$BIN" | sed -n '1,40p' >&2 || true
  exit 1
fi

IFS='.' read -r major minor _ <<< "$minos"

echo "binary: $BIN"
echo "declared minos: $minos"
echo "allowed max:    $MAX_MAJOR.$MAX_MINOR.x"

if (( major < MAX_MAJOR )); then
  exit 0
elif (( major == MAX_MAJOR )) && (( minor <= MAX_MINOR )); then
  exit 0
fi

echo "FAIL: бинарник требует macOS новее порога" >&2
exit 1
#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="$ROOT/crates/aether-native-pages/src/lib.rs"
if grep -nE '(^|[^[:alnum:]_:])Url::parse\(' "$SRC"; then
  echo 'AETHER_BROWSER_NATIVE_PAGES_URL_SYMBOL=FAIL:unqualified-Url::parse'
  exit 1
fi
if ! grep -q 'url::Url::parse' "$SRC"; then
  echo 'AETHER_BROWSER_NATIVE_PAGES_URL_SYMBOL=FAIL:no-qualified-url-parser'
  exit 1
fi
echo 'AETHER_BROWSER_NATIVE_PAGES_URL_SYMBOL=PASS'

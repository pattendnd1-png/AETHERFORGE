#!/usr/bin/env bash
set -Eeuo pipefail

DL="$HOME/Downloads"
ARCHIVE="$DL/OpenDeck-v2.0.31-FRONTEND-TEST-CLOSURE-SOURCE.tar.xz"
EXPECTED_SHA="df4149fbe5d5ff0ed8c77aa6228bfe97b32da33daa6e47d3e5b48f286e3780d0"
SRC="$DL/OpenDeck-v2.0.31-FRONTEND-TEST-CLOSURE-SOURCE"
REUSE="$DL/.opendeck-v231-node-modules-reuse-$$"

fail(){ printf 'OPENDECK_V231_HIT_IT=FAIL:%s\n' "$1" >&2; exit "${2:-1}"; }
sha(){ sha256sum "$1" | awk '{print $1}'; }

printf '%s\n' 'OPENDECK_V231_HIT_IT=START'
for cmd in sha256sum tar xz; do command -v "$cmd" >/dev/null 2>&1 || fail "REQUIRED_COMMAND_MISSING:$cmd" 2; done
[[ -f "$ARCHIVE" ]] || fail "ARCHIVE_MISSING:$ARCHIVE" 3
actual="$(sha "$ARCHIVE")"
[[ "$actual" == "$EXPECTED_SHA" ]] || fail "ARCHIVE_SHA_MISMATCH:$actual" 4
printf 'OPENDECK_V231_ARCHIVE_SHA256=PASS:%s\n' "$actual"

rm -rf "$REUSE"
if [[ -d "$SRC/apps/opendeck-studio/node_modules" ]]; then
  mv "$SRC/apps/opendeck-studio/node_modules" "$REUSE"
  printf '%s\n' 'OPENDECK_V231_EXISTING_NODE_MODULES=PRESERVED'
fi
rm -rf "$SRC"
tar -xJf "$ARCHIVE" -C "$DL"
[[ -d "$SRC" ]] || fail "EXTRACTED_SOURCE_MISSING" 5
if [[ -d "$REUSE" ]]; then
  mkdir -p "$SRC/apps/opendeck-studio"
  mv "$REUSE" "$SRC/apps/opendeck-studio/node_modules"
fi
chmod +x "$SRC/scripts/qualify-v231-host.sh" "$SRC/scripts/activate-v231-qualified.sh" "$SRC/scripts/check-streamdeck-plus.sh"
OPENDECK_V231_ARCHIVE_SHA="$EXPECTED_SHA" exec bash "$SRC/scripts/qualify-v231-host.sh"

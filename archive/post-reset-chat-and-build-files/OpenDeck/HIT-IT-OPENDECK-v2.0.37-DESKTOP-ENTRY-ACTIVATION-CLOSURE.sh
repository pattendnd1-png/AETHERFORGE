#!/usr/bin/env bash
set -Eeuo pipefail

DL="$HOME/Downloads"
ARCHIVE="$DL/OpenDeck-v2.0.37-DESKTOP-ENTRY-ACTIVATION-CLOSURE-SOURCE.tar.xz"
EXPECTED_SHA="178a2bed2d2405a89b1248c4ca78533b34177528b171c2a79b94a6ca5b19d992"
SRC="$DL/OpenDeck-v2.0.37-DESKTOP-ENTRY-ACTIVATION-CLOSURE-SOURCE"
REUSE="$DL/.opendeck-v237-node-modules-reuse-$$"

fail(){ printf 'OPENDECK_V237_HIT_IT=FAIL:%s\n' "$1" >&2; exit "${2:-1}"; }
sha(){ sha256sum "$1" | awk '{print $1}'; }

printf '%s\n' 'OPENDECK_V237_HIT_IT=START'
for cmd in sha256sum tar xz; do command -v "$cmd" >/dev/null 2>&1 || fail "REQUIRED_COMMAND_MISSING:$cmd" 2; done
[[ -f "$ARCHIVE" ]] || fail "ARCHIVE_MISSING:$ARCHIVE" 3
actual="$(sha "$ARCHIVE")"
[[ "$actual" == "$EXPECTED_SHA" ]] || fail "ARCHIVE_SHA_MISMATCH:$actual" 4
printf 'OPENDECK_V237_ARCHIVE_SHA256=PASS:%s\n' "$actual"

rm -rf "$REUSE"
if [[ -d "$SRC/apps/opendeck-studio/node_modules" ]]; then
  mv "$SRC/apps/opendeck-studio/node_modules" "$REUSE"
  printf '%s\n' 'OPENDECK_V237_EXISTING_NODE_MODULES=PRESERVED'
fi
rm -rf "$SRC"
tar -xJf "$ARCHIVE" -C "$DL"
[[ -d "$SRC" ]] || fail "EXTRACTED_SOURCE_MISSING" 5
if [[ -d "$REUSE" ]]; then
  mkdir -p "$SRC/apps/opendeck-studio"
  mv "$REUSE" "$SRC/apps/opendeck-studio/node_modules"
fi
chmod +x "$SRC/scripts/qualify-v237-host.sh" "$SRC/scripts/activate-v237-qualified.sh" "$SRC/scripts/check-v237-menu-launch.sh" "$SRC/scripts/check-streamdeck-plus.sh"
OPENDECK_V237_ARCHIVE_SHA="$EXPECTED_SHA" exec bash "$SRC/scripts/qualify-v237-host.sh"

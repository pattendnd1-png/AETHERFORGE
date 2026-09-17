#!/usr/bin/env bash
set -Eeuo pipefail

DL="$HOME/Downloads"
ARCHIVE="$DL/OpenDeck-v2.0.34-VISIBLE-STARTUP-KDE-LAUNCHER-CLOSURE-SOURCE.tar.xz"
EXPECTED_SHA="180d17d034166bc027f14d77c2a0bfd49a6408bc567e7107bbad36108c731b76"
SRC="$DL/OpenDeck-v2.0.34-VISIBLE-STARTUP-KDE-LAUNCHER-CLOSURE-SOURCE"
REUSE="$DL/.opendeck-v234-node-modules-reuse-$$"

fail(){ printf 'OPENDECK_V234_HIT_IT=FAIL:%s\n' "$1" >&2; exit "${2:-1}"; }
sha(){ sha256sum "$1" | awk '{print $1}'; }

printf '%s\n' 'OPENDECK_V234_HIT_IT=START'
for cmd in sha256sum tar xz; do command -v "$cmd" >/dev/null 2>&1 || fail "REQUIRED_COMMAND_MISSING:$cmd" 2; done
[[ -f "$ARCHIVE" ]] || fail "ARCHIVE_MISSING:$ARCHIVE" 3
actual="$(sha "$ARCHIVE")"
[[ "$actual" == "$EXPECTED_SHA" ]] || fail "ARCHIVE_SHA_MISMATCH:$actual" 4
printf 'OPENDECK_V234_ARCHIVE_SHA256=PASS:%s\n' "$actual"

rm -rf "$REUSE"
if [[ -d "$SRC/apps/opendeck-studio/node_modules" ]]; then
  mv "$SRC/apps/opendeck-studio/node_modules" "$REUSE"
  printf '%s\n' 'OPENDECK_V234_EXISTING_NODE_MODULES=PRESERVED'
fi
rm -rf "$SRC"
tar -xJf "$ARCHIVE" -C "$DL"
[[ -d "$SRC" ]] || fail "EXTRACTED_SOURCE_MISSING" 5
if [[ -d "$REUSE" ]]; then
  mkdir -p "$SRC/apps/opendeck-studio"
  mv "$REUSE" "$SRC/apps/opendeck-studio/node_modules"
fi
chmod +x "$SRC/scripts/qualify-v234-host.sh" "$SRC/scripts/activate-v234-qualified.sh" "$SRC/scripts/check-v234-menu-launch.sh" "$SRC/scripts/check-streamdeck-plus.sh"
OPENDECK_V234_ARCHIVE_SHA="$EXPECTED_SHA" exec bash "$SRC/scripts/qualify-v234-host.sh"

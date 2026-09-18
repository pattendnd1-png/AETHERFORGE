#!/usr/bin/env bash
set -Eeuo pipefail

DL="$HOME/Downloads"
ARCHIVE="$DL/OpenDeck-v2.0.45-FLUID-WINDOW-FILL-CLOSURE-SOURCE.tar.xz"
EXPECTED_SHA="90c0edcff50b976b3421f8abe9dd257e1041b4cbac3bf8741e7fa28c302a6657"
SRC="$DL/OpenDeck-v2.0.45-FLUID-WINDOW-FILL-CLOSURE-SOURCE"
REUSE="$DL/.opendeck-v245-node-modules-reuse-$$"

fail(){ printf 'OPENDECK_V245_HIT_IT=FAIL:%s\n' "$1" >&2; exit "${2:-1}"; }
sha(){ sha256sum "$1" | awk '{print $1}'; }

printf '%s\n' 'OPENDECK_V245_HIT_IT=START'
printf '%s\n' 'OPENDECK_V245_RELEASE=FLUID_WINDOW_FILL_CLOSURE'
printf '%s\n' 'OPENDECK_V245_INSTALL_SCOPE=SYSTEM_WIDE'
printf '%s\n' 'OPENDECK_V245_SUDO=REQUIRED_FOR_FINAL_SYSTEM_INSTALL'
for cmd in sha256sum tar xz sudo; do command -v "$cmd" >/dev/null 2>&1 || fail "REQUIRED_COMMAND_MISSING:$cmd" 2; done
[[ -f "$ARCHIVE" ]] || fail "ARCHIVE_MISSING:$ARCHIVE" 3
actual="$(sha "$ARCHIVE")"
[[ "$actual" == "$EXPECTED_SHA" ]] || fail "ARCHIVE_SHA_MISMATCH:$actual" 4
printf 'OPENDECK_V245_ARCHIVE_SHA256=PASS:%s\n' "$actual"

rm -rf "$REUSE"
if [[ -d "$SRC/apps/opendeck-studio/node_modules" ]]; then
  mv "$SRC/apps/opendeck-studio/node_modules" "$REUSE"
  printf '%s\n' 'OPENDECK_V245_EXISTING_NODE_MODULES=PRESERVED'
fi
rm -rf "$SRC"
tar -xJf "$ARCHIVE" -C "$DL"
[[ -d "$SRC" ]] || fail "EXTRACTED_SOURCE_MISSING" 5
if [[ -d "$REUSE" ]]; then
  mkdir -p "$SRC/apps/opendeck-studio"
  mv "$REUSE" "$SRC/apps/opendeck-studio/node_modules"
fi
chmod +x   "$SRC/scripts/qualify-v245-host.sh"   "$SRC/scripts/stage-v245-system-wide.sh"   "$SRC/scripts/activate-v245-system-wide.sh"   "$SRC/scripts/rollback-v245-system-wide.sh"   "$SRC/scripts/uninstall-v245-system-wide.sh"   "$SRC/scripts/check-v245-system-menu-launch.sh"   "$SRC/scripts/check-streamdeck-plus.sh"
OPENDECK_V245_ARCHIVE_SHA="$EXPECTED_SHA" exec bash "$SRC/scripts/qualify-v245-host.sh"

#!/usr/bin/env bash
set -Eeuo pipefail

DL="$HOME/Downloads"
ARCHIVE="$DL/OpenDeck-v2.0.51-PLUGIN-PACK-INSTALL-ACTIVATION-CLIPPY-CLOSURE-SOURCE.tar.xz"
EXPECTED_SHA="a5d6754a7158eaac4e82d207d8a244b9bf66aa206e55122236405aa61dd3b3b2"
SRC="$DL/OpenDeck-v2.0.51-PLUGIN-PACK-INSTALL-ACTIVATION-CLIPPY-CLOSURE-SOURCE"
REUSE="$DL/.opendeck-v251-node-modules-reuse-$$"

fail(){ printf 'OPENDECK_V251_HIT_IT=FAIL:%s\n' "$1" >&2; exit "${2:-1}"; }
sha(){ sha256sum "$1" | awk '{print $1}'; }

printf '%s\n' 'OPENDECK_V251_HIT_IT=START'
printf '%s\n' 'OPENDECK_V251_RELEASE=PLUGIN_PACK_INSTALL_ACTIVATION_CLIPPY_CLOSURE'
printf '%s\n' 'OPENDECK_V251_INSTALL_SCOPE=SYSTEM_WIDE'
printf '%s\n' 'OPENDECK_V251_SUDO=REQUIRED_FOR_FINAL_SYSTEM_INSTALL'
for cmd in sha256sum tar xz sudo; do command -v "$cmd" >/dev/null 2>&1 || fail "REQUIRED_COMMAND_MISSING:$cmd" 2; done
[[ -f "$ARCHIVE" ]] || fail "ARCHIVE_MISSING:$ARCHIVE" 3
actual="$(sha "$ARCHIVE")"
[[ "$actual" == "$EXPECTED_SHA" ]] || fail "ARCHIVE_SHA_MISMATCH:$actual" 4
printf 'OPENDECK_V251_ARCHIVE_SHA256=PASS:%s\n' "$actual"

rm -rf "$REUSE"
if [[ -d "$SRC/apps/opendeck-studio/node_modules" ]]; then
  mv "$SRC/apps/opendeck-studio/node_modules" "$REUSE"
  printf '%s\n' 'OPENDECK_V251_EXISTING_NODE_MODULES=PRESERVED'
fi
rm -rf "$SRC"
tar -xJf "$ARCHIVE" -C "$DL"
[[ -d "$SRC" ]] || fail 'EXTRACTED_SOURCE_MISSING' 5
if [[ -d "$REUSE" ]]; then
  mkdir -p "$SRC/apps/opendeck-studio"
  mv "$REUSE" "$SRC/apps/opendeck-studio/node_modules"
fi
chmod +x   "$SRC/scripts/qualify-v251-host.sh"   "$SRC/scripts/stage-v251-system-wide.sh"   "$SRC/scripts/activate-v251-system-wide.sh"   "$SRC/scripts/rollback-v251-system-wide.sh"   "$SRC/scripts/uninstall-v251-system-wide.sh"   "$SRC/scripts/check-v251-system-menu-launch.sh"   "$SRC/scripts/check-streamdeck-plus.sh"
OPENDECK_V251_ARCHIVE_SHA="$EXPECTED_SHA" exec bash "$SRC/scripts/qualify-v251-host.sh"

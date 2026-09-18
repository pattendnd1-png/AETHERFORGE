#!/usr/bin/env bash
set -Eeuo pipefail

DL="$HOME/Downloads"
ARCHIVE="$DL/OpenDeck-v2.0.49-DEVICE-ICON-PLUGIN-INSTALL-OBS-TOGGLE-CLOSURE-SOURCE.tar.xz"
EXPECTED_SHA="29cf2a30d246500c67d5e601338db672d2b3ce41867cc7eba5abe0edbee6897e"
SRC="$DL/OpenDeck-v2.0.49-DEVICE-ICON-PLUGIN-INSTALL-OBS-TOGGLE-CLOSURE-SOURCE"
REUSE="$DL/.opendeck-v249-node-modules-reuse-$$"

fail(){ printf 'OPENDECK_V249_HIT_IT=FAIL:%s\n' "$1" >&2; exit "${2:-1}"; }
sha(){ sha256sum "$1" | awk '{print $1}'; }

printf '%s\n' 'OPENDECK_V249_HIT_IT=START'
printf '%s\n' 'OPENDECK_V249_RELEASE=DEVICE_ICON_PLUGIN_INSTALL_OBS_TOGGLE_CLOSURE'
printf '%s\n' 'OPENDECK_V249_INSTALL_SCOPE=SYSTEM_WIDE'
printf '%s\n' 'OPENDECK_V249_SUDO=REQUIRED_FOR_FINAL_SYSTEM_INSTALL'
for cmd in sha256sum tar xz sudo; do command -v "$cmd" >/dev/null 2>&1 || fail "REQUIRED_COMMAND_MISSING:$cmd" 2; done
[[ -f "$ARCHIVE" ]] || fail "ARCHIVE_MISSING:$ARCHIVE" 3
actual="$(sha "$ARCHIVE")"
[[ "$actual" == "$EXPECTED_SHA" ]] || fail "ARCHIVE_SHA_MISMATCH:$actual" 4
printf 'OPENDECK_V249_ARCHIVE_SHA256=PASS:%s\n' "$actual"

rm -rf "$REUSE"
if [[ -d "$SRC/apps/opendeck-studio/node_modules" ]]; then
  mv "$SRC/apps/opendeck-studio/node_modules" "$REUSE"
  printf '%s\n' 'OPENDECK_V249_EXISTING_NODE_MODULES=PRESERVED'
fi
rm -rf "$SRC"
tar -xJf "$ARCHIVE" -C "$DL"
[[ -d "$SRC" ]] || fail 'EXTRACTED_SOURCE_MISSING' 5
if [[ -d "$REUSE" ]]; then
  mkdir -p "$SRC/apps/opendeck-studio"
  mv "$REUSE" "$SRC/apps/opendeck-studio/node_modules"
fi
chmod +x   "$SRC/scripts/qualify-v249-host.sh"   "$SRC/scripts/stage-v249-system-wide.sh"   "$SRC/scripts/activate-v249-system-wide.sh"   "$SRC/scripts/rollback-v249-system-wide.sh"   "$SRC/scripts/uninstall-v249-system-wide.sh"   "$SRC/scripts/check-v249-system-menu-launch.sh"   "$SRC/scripts/check-streamdeck-plus.sh"
OPENDECK_V249_ARCHIVE_SHA="$EXPECTED_SHA" exec bash "$SRC/scripts/qualify-v249-host.sh"

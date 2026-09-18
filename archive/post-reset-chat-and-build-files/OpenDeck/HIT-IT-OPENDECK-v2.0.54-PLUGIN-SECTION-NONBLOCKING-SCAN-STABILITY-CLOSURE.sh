#!/usr/bin/env bash
set -Eeuo pipefail

DL="$HOME/Downloads"
ARCHIVE="$DL/OpenDeck-v2.0.54-PLUGIN-SECTION-NONBLOCKING-SCAN-STABILITY-CLOSURE-SOURCE.tar.xz"
EXPECTED_SHA="54fc2dfe010b8e2f18db5c8ee5d733b24476cc18b7bb3b2823e8443535b4f141"
SRC="$DL/OpenDeck-v2.0.54-PLUGIN-SECTION-NONBLOCKING-SCAN-STABILITY-CLOSURE-SOURCE"
REUSE="$DL/.opendeck-v254-node-modules-reuse-$$"

fail(){ printf 'OPENDECK_V254_HIT_IT=FAIL:%s\n' "$1" >&2; exit "${2:-1}"; }
sha(){ sha256sum "$1" | awk '{print $1}'; }

printf '%s\n' 'OPENDECK_V254_HIT_IT=START'
printf '%s\n' 'OPENDECK_V254_RELEASE=PLUGIN_SECTION_NONBLOCKING_SCAN_STABILITY_CLOSURE'
printf '%s\n' 'OPENDECK_V254_INSTALL_SCOPE=SYSTEM_WIDE'
printf '%s\n' 'OPENDECK_V254_SUDO=REQUIRED_FOR_FINAL_SYSTEM_INSTALL'
for cmd in sha256sum tar xz sudo; do command -v "$cmd" >/dev/null 2>&1 || fail "REQUIRED_COMMAND_MISSING:$cmd" 2; done
[[ -f "$ARCHIVE" ]] || fail "ARCHIVE_MISSING:$ARCHIVE" 3
actual="$(sha "$ARCHIVE")"
[[ "$actual" == "$EXPECTED_SHA" ]] || fail "ARCHIVE_SHA_MISMATCH:$actual" 4
printf 'OPENDECK_V254_ARCHIVE_SHA256=PASS:%s\n' "$actual"

rm -rf "$REUSE"
if [[ -d "$SRC/apps/opendeck-studio/node_modules" ]]; then
  mv "$SRC/apps/opendeck-studio/node_modules" "$REUSE"
  printf '%s\n' 'OPENDECK_V254_EXISTING_NODE_MODULES=PRESERVED'
fi
rm -rf "$SRC"
tar -xJf "$ARCHIVE" -C "$DL"
[[ -d "$SRC" ]] || fail 'EXTRACTED_SOURCE_MISSING' 5
if [[ -d "$REUSE" ]]; then
  mkdir -p "$SRC/apps/opendeck-studio"
  mv "$REUSE" "$SRC/apps/opendeck-studio/node_modules"
fi
chmod +x \
  "$SRC/scripts/qualify-v254-host.sh" \
  "$SRC/scripts/stage-v254-system-wide.sh" \
  "$SRC/scripts/activate-v254-system-wide.sh" \
  "$SRC/scripts/rollback-v254-system-wide.sh" \
  "$SRC/scripts/uninstall-v254-system-wide.sh" \
  "$SRC/scripts/check-v254-system-menu-launch.sh" \
  "$SRC/scripts/check-streamdeck-plus.sh"
OPENDECK_V254_ARCHIVE_SHA="$EXPECTED_SHA" exec bash "$SRC/scripts/qualify-v254-host.sh"

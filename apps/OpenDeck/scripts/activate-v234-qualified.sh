#!/usr/bin/env bash
set -Eeuo pipefail

TARGET_VERSION="2.0.34"
QUALIFIED_BIN="${1:?qualified binary path required}"
EXPECTED_SHA="${2:?expected qualified binary sha256 required}"
SOURCE_ROOT="${3:?source root required}"
ROLLBACK_FILE="${4:?rollback file required}"
VERIFY_FILE="${5:-}"

LIB_ROOT="$HOME/.local/lib"
USER_BIN="$HOME/.local/bin"
INSTALL_ROOT="$LIB_ROOT/opendeck-v$TARGET_VERSION"
DESKTOP_DIR="$HOME/.local/share/applications"
ICON_DIR="$HOME/.local/share/icons/hicolor/256x256/apps"
DESKTOP_FILE="$DESKTOP_DIR/opendeck-studio.desktop"
LEGACY_DESKTOP_FILE="$DESKTOP_DIR/opendeck.desktop"
ICON_SOURCE="$SOURCE_ROOT/apps/opendeck-studio/src-tauri/icons/icon.png"
ICON_DEST="$ICON_DIR/opendeck-studio.png"

say(){ printf '%s\n' "$*"; [[ -n "$VERIFY_FILE" ]] && printf '%s\n' "$*" >> "$VERIFY_FILE" || true; }
fail(){ say "OPENDECK_V234_ACTIVATION=FAIL:${1}"; exit "${2:-1}"; }
sha(){ sha256sum "$1" | awk '{print $1}'; }

for cmd in sha256sum install ln mv rm mkdir desktop-file-validate; do command -v "$cmd" >/dev/null 2>&1 || fail "REQUIRED_COMMAND_MISSING:$cmd" 2; done
[[ -x "$QUALIFIED_BIN" ]] || fail "QUALIFIED_BINARY_MISSING" 3
[[ "$(sha "$QUALIFIED_BIN")" == "$EXPECTED_SHA" ]] || fail "QUALIFIED_BINARY_SHA_MISMATCH" 4
[[ -f "$ICON_SOURCE" ]] || fail "ICON_SOURCE_MISSING" 5
mkdir -p "$LIB_ROOT" "$USER_BIN" "$DESKTOP_DIR" "$ICON_DIR" "$(dirname "$ROLLBACK_FILE")"

previous_target="$(readlink -f "$USER_BIN/opendeck-studio" 2>/dev/null || true)"
previous_sha=""
if [[ -n "$previous_target" && -x "$previous_target" ]]; then previous_sha="$(sha "$previous_target")"; fi
[[ -n "$previous_target" && -x "$previous_target" ]] || fail "CURRENT_OPENDECK_BASELINE_MISSING" 6

{
  printf 'PREVIOUS_ACTIVE_TARGET=%s\n' "$previous_target"
  printf 'PREVIOUS_ACTIVE_SHA256=%s\n' "$previous_sha"
  printf 'NEW_INSTALL_ROOT=%s\n' "$INSTALL_ROOT"
  printf "RESTORE_COMMAND=ln -sfn '%s' '%s' && ln -sfn '%s' '%s'\n" \
    "$previous_target" "$USER_BIN/opendeck-studio" "$previous_target" "$USER_BIN/opendeck"
} > "$ROLLBACK_FILE"
chmod 600 "$ROLLBACK_FILE"

stage="$LIB_ROOT/.opendeck-v${TARGET_VERSION}-activate-$$"
rm -rf -- "$stage"
mkdir -p "$stage/bin"
install -m 0755 "$QUALIFIED_BIN" "$stage/bin/opendeck-studio"
[[ "$(sha "$stage/bin/opendeck-studio")" == "$EXPECTED_SHA" ]] || fail "ACTIVATION_STAGE_SHA_MISMATCH" 7
rm -rf -- "$INSTALL_ROOT"
mv "$stage" "$INSTALL_ROOT"

# Prepare and validate desktop integration before switching the active binary.
install -m 0644 "$ICON_SOURCE" "$ICON_DEST"
tmp_desktop="$DESKTOP_DIR/.opendeck-studio.desktop.v234-$$"
cat > "$tmp_desktop" <<EOF
[Desktop Entry]
Type=Application
Name=OpenDeck+
Comment=Linux-native Stream Deck control center
Exec=$USER_BIN/opendeck-studio
Icon=opendeck-studio
Terminal=false
Categories=Utility;
StartupNotify=true
EOF
chmod 0644 "$tmp_desktop"
desktop-file-validate "$tmp_desktop" || fail "DESKTOP_ENTRY_INVALID" 10

tmp_legacy="$DESKTOP_DIR/.opendeck.desktop.v234-$$"
cat > "$tmp_legacy" <<EOF
[Desktop Entry]
Type=Application
Name=OpenDeck
Comment=Compatibility launcher for OpenDeck+
Exec=$USER_BIN/opendeck-studio
Icon=opendeck-studio
Terminal=false
Categories=Utility;
NoDisplay=true
StartupNotify=true
EOF
chmod 0644 "$tmp_legacy"
desktop-file-validate "$tmp_legacy" || fail "LEGACY_DESKTOP_ENTRY_INVALID" 10

mv -f "$tmp_desktop" "$DESKTOP_FILE"
mv -f "$tmp_legacy" "$LEGACY_DESKTOP_FILE"
command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$DESKTOP_DIR" >/dev/null 2>&1 || true
if command -v kbuildsycoca6 >/dev/null 2>&1; then
  kbuildsycoca6 --noincremental >/dev/null 2>&1 || fail "KDE_CACHE_REFRESH_FAILED" 10
elif command -v kbuildsycoca5 >/dev/null 2>&1; then
  kbuildsycoca5 --noincremental >/dev/null 2>&1 || fail "KDE_CACHE_REFRESH_FAILED" 10
fi

# All activation prerequisites are green. Switch the user launch aliases last.
for link_name in opendeck-studio opendeck; do
  tmp_link="$USER_BIN/.${link_name}.v234-$$"
  rm -f -- "$tmp_link"
  ln -s "$INSTALL_ROOT/bin/opendeck-studio" "$tmp_link"
  mv -Tf "$tmp_link" "$USER_BIN/$link_name"
done
[[ "$(sha "$USER_BIN/opendeck-studio")" == "$EXPECTED_SHA" ]] || fail "ACTIVE_BINARY_SHA_MISMATCH" 8

# Preserve the immediately previous install exactly as-is for rollback. Do not
# clean historical installs in this feature cutover.
[[ -x "$INSTALL_ROOT/bin/opendeck-studio" ]] || fail "INSTALL_ROOT_MISSING" 9
[[ "$(readlink -f "$USER_BIN/opendeck-studio")" == "$INSTALL_ROOT/bin/opendeck-studio" ]] || fail "ACTIVE_SYMLINK_TARGET_MISMATCH" 9
[[ -x "$previous_target" ]] || fail "ROLLBACK_TARGET_LOST" 9

say "OPENDECK_V234_REPLACEMENT_INSTALL=PASS"
say "OPENDECK_V234_INSTALL_ROOT=$INSTALL_ROOT"
say "OPENDECK_V234_PREVIOUS_ACTIVE_TARGET=$previous_target"
say "OPENDECK_V234_PREVIOUS_ACTIVE_SHA256=$previous_sha"
say "OPENDECK_V234_ACTIVE_BINARY_SHA256=$(sha "$USER_BIN/opendeck-studio")"
say "OPENDECK_V234_ROLLBACK_FILE=$ROLLBACK_FILE"
say "OPENDECK_V234_BACKGROUND_SERVICES=NONE"
say "OPENDECK_V234_AUTOSTART=DISABLED"

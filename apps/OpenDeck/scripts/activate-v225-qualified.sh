#!/usr/bin/env bash
set -Eeuo pipefail

TARGET_VERSION="2.0.25"
ROLLBACK_KEEP_VERSION="2.0.8"

QUALIFIED_BIN="${1:?qualified binary path required}"
EXPECTED_SHA="${2:?expected qualified binary sha256 required}"
SOURCE_ROOT="${3:?source root required}"
ROLLBACK_FILE="${4:?rollback file required}"
VERIFY_FILE="${5:-}"

LIB_ROOT="$HOME/.local/lib"
USER_BIN="$HOME/.local/bin"
INSTALL_ROOT="$LIB_ROOT/opendeck-v$TARGET_VERSION"
ROLLBACK_ROOT="$LIB_ROOT/opendeck-v$ROLLBACK_KEEP_VERSION"
DESKTOP_DIR="$HOME/.local/share/applications"
ICON_DIR="$HOME/.local/share/icons/hicolor/256x256/apps"
DESKTOP_FILE="$DESKTOP_DIR/opendeck-studio.desktop"
ICON_SOURCE="$SOURCE_ROOT/apps/opendeck-studio/src-tauri/icons/icon.png"
ICON_DEST="$ICON_DIR/opendeck-studio.png"

say(){ printf '%s\n' "$*"; [[ -n "$VERIFY_FILE" ]] && printf '%s\n' "$*" >> "$VERIFY_FILE" || true; }
fail(){ say "OPENDECK_V225_ACTIVATION=FAIL:${1}"; exit "${2:-1}"; }
sha(){ sha256sum "$1" | awk '{print $1}'; }

for cmd in sha256sum install ln mv rm mkdir find; do command -v "$cmd" >/dev/null 2>&1 || fail "REQUIRED_COMMAND_MISSING:$cmd" 2; done
[[ -x "$QUALIFIED_BIN" ]] || fail "QUALIFIED_BINARY_MISSING" 3
[[ "$(sha "$QUALIFIED_BIN")" == "$EXPECTED_SHA" ]] || fail "QUALIFIED_BINARY_SHA_MISMATCH" 4
[[ -f "$ICON_SOURCE" ]] || fail "ICON_SOURCE_MISSING" 5

mkdir -p "$LIB_ROOT" "$USER_BIN" "$DESKTOP_DIR" "$ICON_DIR" "$(dirname "$ROLLBACK_FILE")"

previous_target="$(readlink -f "$USER_BIN/opendeck-studio" 2>/dev/null || true)"
previous_sha=""
if [[ -n "$previous_target" && -x "$previous_target" ]]; then previous_sha="$(sha "$previous_target")"; fi

# Preserve one known-good rollback install. First cutover intentionally keeps 2.0.8.
[[ -x "$ROLLBACK_ROOT/bin/opendeck-studio" ]] || fail "ROLLBACK_V${ROLLBACK_KEEP_VERSION}_MISSING" 6
rollback_sha="$(sha "$ROLLBACK_ROOT/bin/opendeck-studio")"
{
  printf 'ROLLBACK_VERSION=%s\n' "$ROLLBACK_KEEP_VERSION"
  printf 'ROLLBACK_TARGET=%s\n' "$ROLLBACK_ROOT/bin/opendeck-studio"
  printf 'ROLLBACK_SHA256=%s\n' "$rollback_sha"
  printf 'PREVIOUS_ACTIVE_TARGET=%s\n' "$previous_target"
  printf 'PREVIOUS_ACTIVE_SHA256=%s\n' "$previous_sha"
  printf "RESTORE_COMMAND=ln -sfn '%s' '%s' && ln -sfn '%s' '%s'\n" \
    "$ROLLBACK_ROOT/bin/opendeck-studio" "$USER_BIN/opendeck-studio" \
    "$ROLLBACK_ROOT/bin/opendeck-studio" "$USER_BIN/opendeck"
} > "$ROLLBACK_FILE"
chmod 600 "$ROLLBACK_FILE"

stage="$LIB_ROOT/.opendeck-v${TARGET_VERSION}-activate-$$"
rm -rf -- "$stage"
mkdir -p "$stage/bin"
install -m 0755 "$QUALIFIED_BIN" "$stage/bin/opendeck-studio"
[[ "$(sha "$stage/bin/opendeck-studio")" == "$EXPECTED_SHA" ]] || fail "ACTIVATION_STAGE_SHA_MISMATCH" 7

rm -rf -- "$INSTALL_ROOT"
mv "$stage" "$INSTALL_ROOT"

# Use version-independent launch paths so desktop launchers never need a versioned Exec target.
for link_name in opendeck-studio opendeck; do
  tmp_link="$USER_BIN/.${link_name}.v225-$$"
  rm -f -- "$tmp_link"
  ln -s "$INSTALL_ROOT/bin/opendeck-studio" "$tmp_link"
  mv -Tf "$tmp_link" "$USER_BIN/$link_name"
done
[[ "$(sha "$USER_BIN/opendeck-studio")" == "$EXPECTED_SHA" ]] || fail "ACTIVE_BINARY_SHA_MISMATCH" 8

install -m 0644 "$ICON_SOURCE" "$ICON_DEST"
tmp_desktop="$DESKTOP_DIR/.opendeck-studio.desktop.v225-$$"
cat > "$tmp_desktop" <<EOF
[Desktop Entry]
Type=Application
Name=OpenDeck+
Comment=Linux-native Stream Deck control center
Exec=$USER_BIN/opendeck-studio
Icon=opendeck-studio
Terminal=false
Categories=Utility;AudioVideo;
StartupNotify=true
EOF
chmod 0644 "$tmp_desktop"
mv -f "$tmp_desktop" "$DESKTOP_FILE"
command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$DESKTOP_DIR" >/dev/null 2>&1 || true

cleanup_old_installs(){
  local path base
  shopt -s nullglob
  for path in \
    "$LIB_ROOT"/opendeck-v* \
    "$LIB_ROOT"/.opendeck-v*-qualified \
    "$LIB_ROOT"/.opendeck-v*-stage-* \
    "$LIB_ROOT"/.opendeck-v*-activate-* \
    "$LIB_ROOT"/opendeck-v*-previous-*; do
    [[ -e "$path" || -L "$path" ]] || continue
    [[ "$path" == "$INSTALL_ROOT" || "$path" == "$ROLLBACK_ROOT" ]] && continue
    base="$(basename "$path")"
    [[ "$base" == opendeck-v* || "$base" == .opendeck-v* ]] || continue
    rm -rf -- "$path"
  done
  shopt -u nullglob
}
cleanup_old_installs

# The staged qualification copy is redundant after the canonical install is verified.
rm -rf -- "$LIB_ROOT/.opendeck-v${TARGET_VERSION}-qualified"

[[ -x "$INSTALL_ROOT/bin/opendeck-studio" ]] || fail "INSTALL_ROOT_MISSING_AFTER_CLEANUP" 9
[[ -x "$ROLLBACK_ROOT/bin/opendeck-studio" ]] || fail "ROLLBACK_MISSING_AFTER_CLEANUP" 9
[[ "$(readlink -f "$USER_BIN/opendeck-studio")" == "$INSTALL_ROOT/bin/opendeck-studio" ]] || fail "ACTIVE_SYMLINK_TARGET_MISMATCH" 9
[[ -f "$DESKTOP_FILE" ]] || fail "DESKTOP_ENTRY_MISSING" 9
[[ -f "$ICON_DEST" ]] || fail "ICON_MISSING" 9

say "OPENDECK_V225_REPLACEMENT_INSTALL=PASS"
say "OPENDECK_V225_INSTALL_ROOT=$INSTALL_ROOT"
say "OPENDECK_V225_ROLLBACK_ROOT=$ROLLBACK_ROOT"
say "OPENDECK_V225_ACTIVE_BINARY_SHA256=$(sha "$USER_BIN/opendeck-studio")"
say "OPENDECK_V225_DESKTOP_ENTRY=$DESKTOP_FILE"
say "OPENDECK_V225_ICON=$ICON_DEST"

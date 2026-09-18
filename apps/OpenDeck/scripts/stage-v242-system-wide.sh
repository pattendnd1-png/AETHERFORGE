#!/usr/bin/env bash
set -Eeuo pipefail

TARGET_VERSION="2.0.42"
QUALIFIED_BIN="${1:?qualified binary path required}"
EXPECTED_SHA="${2:?expected qualified binary sha256 required}"
SOURCE_ROOT="${3:?source root required}"
VERIFY_FILE="${4:-}"

OPT_ROOT="${OPENDECK_OPT_ROOT:-/opt/opendeck-plus}"
INSTALL_ROOT="$OPT_ROOT/$TARGET_VERSION"
SYSTEM_APP_DIR="${OPENDECK_SYSTEM_APP_DIR:-/usr/share/applications}"
SYSTEM_ICON_DIR="${OPENDECK_SYSTEM_ICON_DIR:-/usr/share/icons/hicolor/64x64/apps}"
UDEV_DIR="${OPENDECK_UDEV_DIR:-/etc/udev/rules.d}"
LOCAL_BIN_DIR="${OPENDECK_LOCAL_BIN_DIR:-/usr/local/bin}"
LOCAL_SBIN_DIR="${OPENDECK_LOCAL_SBIN_DIR:-/usr/local/sbin}"
STATE_ROOT="${OPENDECK_STATE_ROOT:-/var/lib/opendeck-plus}"

ICON_SOURCE="$SOURCE_ROOT/apps/opendeck-studio/src-tauri/icons/icon.png"
UDEV_SOURCE="$SOURCE_ROOT/packaging/70-opendeck-streamdeck.rules"
ROLLBACK_SOURCE="$SOURCE_ROOT/scripts/rollback-v242-system-wide.sh"
UNINSTALL_SOURCE="$SOURCE_ROOT/scripts/uninstall-v242-system-wide.sh"

say(){ printf '%s\n' "$*"; [[ -n "$VERIFY_FILE" ]] && printf '%s\n' "$*" >> "$VERIFY_FILE" || true; }
fail(){ say "OPENDECK_V242_SYSTEM_STAGE=FAIL:${1}"; exit "${2:-1}"; }
sha(){ sha256sum "$1" | awk '{print $1}'; }

if [[ ${EUID:-$(id -u)} -ne 0 && "${OPENDECK_ALLOW_NONROOT:-0}" != "1" ]]; then
  fail "ROOT_REQUIRED" 2
fi
for cmd in sha256sum install mkdir mv rm find sort xargs desktop-file-validate; do
  command -v "$cmd" >/dev/null 2>&1 || fail "REQUIRED_COMMAND_MISSING:$cmd" 2
done
[[ -x "$QUALIFIED_BIN" ]] || fail "QUALIFIED_BINARY_MISSING" 3
[[ "$(sha "$QUALIFIED_BIN")" == "$EXPECTED_SHA" ]] || fail "QUALIFIED_BINARY_SHA_MISMATCH" 4
[[ -f "$ICON_SOURCE" ]] || fail "ICON_SOURCE_MISSING" 5
[[ -f "$UDEV_SOURCE" ]] || fail "UDEV_SOURCE_MISSING" 5
[[ -x "$ROLLBACK_SOURCE" ]] || fail "ROLLBACK_HELPER_MISSING" 5
[[ -x "$UNINSTALL_SOURCE" ]] || fail "UNINSTALL_HELPER_MISSING" 5

mkdir -p "$OPT_ROOT" "$STATE_ROOT"
stage="$OPT_ROOT/.opendeck-v${TARGET_VERSION}-stage-$$"
rm -rf -- "$stage"
mkdir -p \
  "$stage/bin" \
  "$stage/share/applications" \
  "$stage/share/icons/hicolor/64x64/apps" \
  "$stage/share/udev" \
  "$stage/share/doc/opendeck-plus" \
  "$stage/libexec" \
  "$stage/manifest"

install -m 0755 "$QUALIFIED_BIN" "$stage/bin/opendeck-studio"
install -m 0644 "$ICON_SOURCE" "$stage/share/icons/hicolor/64x64/apps/opendeck-studio.png"
install -m 0644 "$UDEV_SOURCE" "$stage/share/udev/70-opendeck-streamdeck.rules"
install -m 0755 "$ROLLBACK_SOURCE" "$stage/libexec/opendeck-rollback"
install -m 0755 "$UNINSTALL_SOURCE" "$stage/libexec/opendeck-uninstall"
install -m 0644 "$SOURCE_ROOT/README.md" "$stage/share/doc/opendeck-plus/README.md"
printf '%s\n' "$TARGET_VERSION" > "$stage/VERSION"

cat > "$stage/share/applications/opendeck-studio.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=OpenDeck+
Comment=Linux-native Stream Deck control center
Exec=$LOCAL_BIN_DIR/opendeck-studio
Icon=opendeck-studio
Terminal=false
Categories=Utility;
StartupNotify=true
EOF
chmod 0644 "$stage/share/applications/opendeck-studio.desktop"
desktop-file-validate "$stage/share/applications/opendeck-studio.desktop" || fail "DESKTOP_TEMPLATE_INVALID" 6

cat > "$stage/manifest/INSTALL-MANIFEST.txt" <<EOF
OPENDECK_PRODUCT=OpenDeck+
OPENDECK_VERSION=$TARGET_VERSION
OPENDECK_INSTALL_ROOT=$INSTALL_ROOT
OPENDECK_CURRENT_LINK=$OPT_ROOT/current
OPENDECK_COMMAND=$LOCAL_BIN_DIR/opendeck-studio
OPENDECK_DESKTOP=$SYSTEM_APP_DIR/opendeck-studio.desktop
OPENDECK_ICON=$SYSTEM_ICON_DIR/opendeck-studio.png
OPENDECK_UDEV=$UDEV_DIR/70-opendeck-streamdeck.rules
OPENDECK_UNINSTALL=$LOCAL_SBIN_DIR/opendeck-uninstall
OPENDECK_ROLLBACK=$LOCAL_SBIN_DIR/opendeck-rollback
OPENDECK_STATE_ROOT=$STATE_ROOT
OPENDECK_BINARY_SHA256=$EXPECTED_SHA
OPENDECK_BACKGROUND_SERVICES=NONE
OPENDECK_AUTOSTART=DISABLED
EOF

(
  cd "$stage"
  find bin libexec share VERSION manifest/INSTALL-MANIFEST.txt -type f -print0 \
    | sort -z \
    | xargs -0 sha256sum > manifest/SHA256SUMS.txt
)
(
  cd "$stage"
  sha256sum -c manifest/SHA256SUMS.txt >/dev/null
) || fail "PAYLOAD_CHECKSUM_VERIFY_FAILED" 7
[[ "$(sha "$stage/bin/opendeck-studio")" == "$EXPECTED_SHA" ]] || fail "STAGED_BINARY_SHA_MISMATCH" 7

rm -rf -- "$INSTALL_ROOT"
mv "$stage" "$INSTALL_ROOT"
[[ -x "$INSTALL_ROOT/bin/opendeck-studio" ]] || fail "INSTALL_ROOT_BINARY_MISSING" 8
[[ "$(sha "$INSTALL_ROOT/bin/opendeck-studio")" == "$EXPECTED_SHA" ]] || fail "INSTALL_ROOT_BINARY_SHA_MISMATCH" 8
(
  cd "$INSTALL_ROOT"
  sha256sum -c manifest/SHA256SUMS.txt >/dev/null
) || fail "INSTALL_ROOT_PAYLOAD_CHECKSUM_FAILED" 8

say "OPENDECK_V242_SYSTEM_STAGE=PASS"
say "OPENDECK_V242_SYSTEM_INSTALL_ROOT=$INSTALL_ROOT"
say "OPENDECK_V242_SYSTEM_BINARY_SHA256=$(sha "$INSTALL_ROOT/bin/opendeck-studio")"
say "OPENDECK_V242_SYSTEM_MANIFEST=$INSTALL_ROOT/manifest/INSTALL-MANIFEST.txt"
say "OPENDECK_V242_SYSTEM_SHA256SUMS=$INSTALL_ROOT/manifest/SHA256SUMS.txt"

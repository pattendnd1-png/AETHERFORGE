#!/usr/bin/env bash
set -Eeuo pipefail

TARGET_VERSION="2.0.42"
EXPECTED_SHA="${1:?expected staged binary sha256 required}"
TARGET_USER="${2:?target desktop user required}"
TARGET_HOME="${3:?target user home required}"
ROLLBACK_FILE="${4:?user rollback file required}"
VERIFY_FILE="${5:-}"

OPT_ROOT="${OPENDECK_OPT_ROOT:-/opt/opendeck-plus}"
INSTALL_ROOT="$OPT_ROOT/$TARGET_VERSION"
CURRENT_LINK="$OPT_ROOT/current"
LOCAL_BIN_DIR="${OPENDECK_LOCAL_BIN_DIR:-/usr/local/bin}"
LOCAL_SBIN_DIR="${OPENDECK_LOCAL_SBIN_DIR:-/usr/local/sbin}"
SYSTEM_APP_DIR="${OPENDECK_SYSTEM_APP_DIR:-/usr/share/applications}"
SYSTEM_ICON_DIR="${OPENDECK_SYSTEM_ICON_DIR:-/usr/share/icons/hicolor/64x64/apps}"
SYSTEM_ICON_BASE="${OPENDECK_SYSTEM_ICON_BASE:-/usr/share/icons/hicolor}"
UDEV_DIR="${OPENDECK_UDEV_DIR:-/etc/udev/rules.d}"
STATE_ROOT="${OPENDECK_STATE_ROOT:-/var/lib/opendeck-plus}"
ROLLBACK_DIR="$STATE_ROOT/rollback-v$TARGET_VERSION"

SYSTEM_DESKTOP="$SYSTEM_APP_DIR/opendeck-studio.desktop"
SYSTEM_LEGACY_DESKTOP="$SYSTEM_APP_DIR/opendeck.desktop"
SYSTEM_ICON="$SYSTEM_ICON_DIR/opendeck-studio.png"
SYSTEM_UDEV="$UDEV_DIR/70-opendeck-streamdeck.rules"
SYSTEM_BIN="$LOCAL_BIN_DIR/opendeck-studio"
SYSTEM_BIN_ALIAS="$LOCAL_BIN_DIR/opendeck"
SYSTEM_ROLLBACK="$LOCAL_SBIN_DIR/opendeck-rollback"
SYSTEM_UNINSTALL="$LOCAL_SBIN_DIR/opendeck-uninstall"
USER_BIN="$TARGET_HOME/.local/bin/opendeck-studio"
USER_BIN_ALIAS="$TARGET_HOME/.local/bin/opendeck"
USER_DESKTOP="$TARGET_HOME/.local/share/applications/opendeck-studio.desktop"
USER_LEGACY_DESKTOP="$TARGET_HOME/.local/share/applications/opendeck.desktop"

say(){ printf '%s\n' "$*"; [[ -n "$VERIFY_FILE" ]] && printf '%s\n' "$*" >> "$VERIFY_FILE" || true; }
fail(){ say "OPENDECK_V242_ACTIVATION=FAIL:${1}"; exit "${2:-1}"; }
sha(){ sha256sum "$1" | awk '{print $1}'; }
ROLLBACK_ARMED=0
rollback_on_failure(){
  local rc=$?
  if [[ $rc -ne 0 && "$ROLLBACK_ARMED" == "1" && -x "$INSTALL_ROOT/libexec/opendeck-rollback" ]]; then
    say "OPENDECK_V242_ACTIVATION_ROLLBACK=START"
    OPENDECK_SKIP_KDE_REFRESH=1 "$INSTALL_ROOT/libexec/opendeck-rollback" "$ROLLBACK_DIR" "$VERIFY_FILE" >/dev/null 2>&1 || true
    say "OPENDECK_V242_ACTIVATION_ROLLBACK=ATTEMPTED"
  fi
  exit "$rc"
}
trap rollback_on_failure EXIT

if [[ ${EUID:-$(id -u)} -ne 0 && "${OPENDECK_ALLOW_NONROOT:-0}" != "1" ]]; then fail "ROOT_REQUIRED" 2; fi
for cmd in sha256sum install ln mv rm mkdir cp desktop-file-validate readlink; do command -v "$cmd" >/dev/null 2>&1 || fail "REQUIRED_COMMAND_MISSING:$cmd" 2; done
[[ -x "$INSTALL_ROOT/bin/opendeck-studio" ]] || fail "STAGED_INSTALL_MISSING" 3
[[ "$(sha "$INSTALL_ROOT/bin/opendeck-studio")" == "$EXPECTED_SHA" ]] || fail "STAGED_BINARY_SHA_MISMATCH" 4
(
  cd "$INSTALL_ROOT"
  sha256sum -c manifest/SHA256SUMS.txt >/dev/null
) || fail "STAGED_PAYLOAD_CHECKSUM_FAILED" 4
desktop-file-validate "$INSTALL_ROOT/share/applications/opendeck-studio.desktop" || fail "STAGED_DESKTOP_INVALID" 4

rm -rf -- "$ROLLBACK_DIR"
mkdir -p "$ROLLBACK_DIR/files" "$STATE_ROOT" "$LOCAL_BIN_DIR" "$LOCAL_SBIN_DIR" "$SYSTEM_APP_DIR" "$SYSTEM_ICON_DIR" "$UDEV_DIR" "$(dirname "$ROLLBACK_FILE")"
: > "$ROLLBACK_DIR/items.tsv"

backup_item(){
  local key="$1" target="$2"
  if [[ -e "$target" || -L "$target" ]]; then
    cp -a "$target" "$ROLLBACK_DIR/files/$key"
    printf '%s\tpresent\t%s\n' "$key" "$target" >> "$ROLLBACK_DIR/items.tsv"
  else
    printf '%s\tabsent\t%s\n' "$key" "$target" >> "$ROLLBACK_DIR/items.tsv"
  fi
}

# Preserve every path that the full install will mutate, including any legacy
# per-user launchers. The currently active versioned /opt tree is never deleted.
PREVIOUS_CURRENT_TARGET="$(readlink -f "$CURRENT_LINK" 2>/dev/null || true)"
backup_item current "$CURRENT_LINK"
backup_item system-bin "$SYSTEM_BIN"
backup_item system-bin-alias "$SYSTEM_BIN_ALIAS"
backup_item system-desktop "$SYSTEM_DESKTOP"
backup_item system-legacy-desktop "$SYSTEM_LEGACY_DESKTOP"
backup_item system-icon "$SYSTEM_ICON"
backup_item system-udev "$SYSTEM_UDEV"
backup_item system-rollback "$SYSTEM_ROLLBACK"
backup_item system-uninstall "$SYSTEM_UNINSTALL"
backup_item user-bin "$USER_BIN"
backup_item user-bin-alias "$USER_BIN_ALIAS"
backup_item user-desktop "$USER_DESKTOP"
backup_item user-legacy-desktop "$USER_LEGACY_DESKTOP"

cat > "$ROLLBACK_DIR/context.env" <<EOF
TARGET_USER=$(printf '%q' "$TARGET_USER")
TARGET_HOME=$(printf '%q' "$TARGET_HOME")
TARGET_VERSION=$(printf '%q' "$TARGET_VERSION")
INSTALL_ROOT=$(printf '%q' "$INSTALL_ROOT")
EOF
chmod 0600 "$ROLLBACK_DIR/context.env" "$ROLLBACK_DIR/items.tsv"
ROLLBACK_ARMED=1

# System-wide cutover. All targets resolve through /opt/opendeck-plus/current.
tmp_current="$OPT_ROOT/.current.v242-$$"
rm -f -- "$tmp_current"
ln -s "$INSTALL_ROOT" "$tmp_current"
mv -Tf "$tmp_current" "$CURRENT_LINK"

for pair in \
  "$SYSTEM_BIN:$CURRENT_LINK/bin/opendeck-studio" \
  "$SYSTEM_BIN_ALIAS:$CURRENT_LINK/bin/opendeck-studio"; do
  dest="${pair%%:*}"; target="${pair#*:}"
  tmp="$LOCAL_BIN_DIR/.$(basename "$dest").v242-$$"
  rm -f -- "$tmp"
  ln -s "$target" "$tmp"
  mv -Tf "$tmp" "$dest"
done

install -m 0644 "$INSTALL_ROOT/share/applications/opendeck-studio.desktop" "$SYSTEM_DESKTOP"
rm -f -- "$SYSTEM_LEGACY_DESKTOP"
install -m 0644 "$INSTALL_ROOT/share/icons/hicolor/64x64/apps/opendeck-studio.png" "$SYSTEM_ICON"
install -m 0644 "$INSTALL_ROOT/share/udev/70-opendeck-streamdeck.rules" "$SYSTEM_UDEV"
install -m 0755 "$INSTALL_ROOT/libexec/opendeck-rollback" "$SYSTEM_ROLLBACK"
install -m 0755 "$INSTALL_ROOT/libexec/opendeck-uninstall" "$SYSTEM_UNINSTALL"
desktop-file-validate "$SYSTEM_DESKTOP" || fail "SYSTEM_DESKTOP_INVALID" 5

cat > "$STATE_ROOT/active.env" <<EOF
OPENDECK_VERSION=$TARGET_VERSION
OPENDECK_INSTALL_ROOT=$INSTALL_ROOT
OPENDECK_BINARY_SHA256=$EXPECTED_SHA
OPENDECK_TARGET_USER=$(printf '%q' "$TARGET_USER")
OPENDECK_TARGET_HOME=$(printf '%q' "$TARGET_HOME")
EOF
chmod 0644 "$STATE_ROOT/active.env"

# REMOVE_USER_SHADOWS_AFTER_SYSTEM_SWITCH: only after the system paths exist and
# validate, remove per-user desktop/bin entries that would shadow the full install.
rm -f -- "$USER_BIN" "$USER_BIN_ALIAS" "$USER_DESKTOP" "$USER_LEGACY_DESKTOP"

command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$SYSTEM_APP_DIR" >/dev/null 2>&1 || true
command -v gtk-update-icon-cache >/dev/null 2>&1 && gtk-update-icon-cache -f -t "$SYSTEM_ICON_BASE" >/dev/null 2>&1 || true
command -v udevadm >/dev/null 2>&1 && udevadm control --reload-rules >/dev/null 2>&1 || true
if [[ "${OPENDECK_SKIP_KDE_REFRESH:-0}" != "1" ]]; then
  if command -v kbuildsycoca6 >/dev/null 2>&1; then
    if [[ ${EUID:-$(id -u)} -eq 0 && -n "$(command -v sudo 2>/dev/null || true)" ]]; then
      sudo -u "$TARGET_USER" env HOME="$TARGET_HOME" kbuildsycoca6 --noincremental >/dev/null 2>&1 || fail "KDE_CACHE_REFRESH_FAILED" 6
    else
      kbuildsycoca6 --noincremental >/dev/null 2>&1 || fail "KDE_CACHE_REFRESH_FAILED" 6
    fi
  elif command -v kbuildsycoca5 >/dev/null 2>&1; then
    if [[ ${EUID:-$(id -u)} -eq 0 && -n "$(command -v sudo 2>/dev/null || true)" ]]; then
      sudo -u "$TARGET_USER" env HOME="$TARGET_HOME" kbuildsycoca5 --noincremental >/dev/null 2>&1 || fail "KDE_CACHE_REFRESH_FAILED" 6
    else
      kbuildsycoca5 --noincremental >/dev/null 2>&1 || fail "KDE_CACHE_REFRESH_FAILED" 6
    fi
  fi
fi

[[ "$(readlink -f "$CURRENT_LINK")" == "$INSTALL_ROOT" ]] || fail "CURRENT_LINK_MISMATCH" 7
[[ "$(readlink -f "$SYSTEM_BIN")" == "$INSTALL_ROOT/bin/opendeck-studio" ]] || fail "SYSTEM_BIN_TARGET_MISMATCH" 7
[[ "$(sha "$SYSTEM_BIN")" == "$EXPECTED_SHA" ]] || fail "SYSTEM_BIN_SHA_MISMATCH" 7
[[ -f "$SYSTEM_DESKTOP" ]] || fail "SYSTEM_DESKTOP_MISSING" 7
[[ ! -e "$USER_DESKTOP" && ! -L "$USER_DESKTOP" ]] || fail "USER_DESKTOP_SHADOW_REMAINS" 7
[[ ! -e "$USER_BIN" && ! -L "$USER_BIN" ]] || fail "USER_BIN_SHADOW_REMAINS" 7

cat > "$ROLLBACK_FILE" <<EOF
OPENDECK_VERSION=$TARGET_VERSION
SYSTEM_INSTALL_ROOT=$INSTALL_ROOT
SYSTEM_ACTIVE_COMMAND=$SYSTEM_BIN
ROLLBACK_STATE=$ROLLBACK_DIR
ROLLBACK_COMMAND=sudo $SYSTEM_ROLLBACK $ROLLBACK_DIR
UNINSTALL_COMMAND=sudo $SYSTEM_UNINSTALL
PREVIOUS_SYSTEM_INSTALL_PRESERVED=$PREVIOUS_CURRENT_TARGET
EOF
chmod 0600 "$ROLLBACK_FILE"
if command -v chown >/dev/null 2>&1 && id "$TARGET_USER" >/dev/null 2>&1; then
  chown "$TARGET_USER:$(id -gn "$TARGET_USER")" "$ROLLBACK_FILE" >/dev/null 2>&1 || true
fi

ROLLBACK_ARMED=0
say "OPENDECK_V242_SYSTEM_FULL_INSTALL=PASS"
say "OPENDECK_V242_INSTALL_ROOT=$INSTALL_ROOT"
say "OPENDECK_V242_CURRENT_LINK=$CURRENT_LINK"
say "OPENDECK_V242_SYSTEM_COMMAND=$SYSTEM_BIN"
say "OPENDECK_V242_SYSTEM_DESKTOP=$SYSTEM_DESKTOP"
say "OPENDECK_V242_ACTIVE_BINARY_SHA256=$(sha "$SYSTEM_BIN")"
say "OPENDECK_V242_ROLLBACK_STATE=$ROLLBACK_DIR"
say "OPENDECK_V242_ROLLBACK_FILE=$ROLLBACK_FILE"
say "OPENDECK_V242_BACKGROUND_SERVICES=NONE"
say "OPENDECK_V242_AUTOSTART=DISABLED"

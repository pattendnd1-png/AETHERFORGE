#!/usr/bin/env bash
set -Eeuo pipefail

TARGET_VERSION="2.0.39"
STATE_ROOT="${OPENDECK_STATE_ROOT:-/var/lib/opendeck-plus}"
SYSTEM_APP_DIR="${OPENDECK_SYSTEM_APP_DIR:-/usr/share/applications}"
SYSTEM_ICON_BASE="${OPENDECK_SYSTEM_ICON_BASE:-/usr/share/icons/hicolor}"
ROLLBACK_DIR="${1:-$STATE_ROOT/rollback-v$TARGET_VERSION}"
VERIFY_FILE="${2:-}"

say(){ printf '%s\n' "$*"; [[ -n "$VERIFY_FILE" ]] && printf '%s\n' "$*" >> "$VERIFY_FILE" || true; }
fail(){ say "OPENDECK_V239_ROLLBACK=FAIL:${1}"; exit "${2:-1}"; }
if [[ ${EUID:-$(id -u)} -ne 0 && "${OPENDECK_ALLOW_NONROOT:-0}" != "1" ]]; then fail "ROOT_REQUIRED" 2; fi
[[ -f "$ROLLBACK_DIR/items.tsv" ]] || fail "ROLLBACK_STATE_MISSING:$ROLLBACK_DIR" 3

remove_target(){
  local target="$1"
  if [[ -L "$target" || -f "$target" ]]; then rm -f -- "$target"
  elif [[ -d "$target" ]]; then rm -rf -- "$target"
  fi
}

while IFS=$'\t' read -r key status target; do
  [[ -n "$key" && -n "$status" && -n "$target" ]] || continue
  remove_target "$target"
  if [[ "$status" == "present" ]]; then
    [[ -e "$ROLLBACK_DIR/files/$key" || -L "$ROLLBACK_DIR/files/$key" ]] || fail "BACKUP_ITEM_MISSING:$key" 4
    mkdir -p "$(dirname "$target")"
    cp -a "$ROLLBACK_DIR/files/$key" "$target"
  elif [[ "$status" != "absent" ]]; then
    fail "ROLLBACK_STATE_INVALID:$key:$status" 4
  fi
done < "$ROLLBACK_DIR/items.tsv"

command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$SYSTEM_APP_DIR" >/dev/null 2>&1 || true
command -v gtk-update-icon-cache >/dev/null 2>&1 && gtk-update-icon-cache -f -t "$SYSTEM_ICON_BASE" >/dev/null 2>&1 || true
command -v udevadm >/dev/null 2>&1 && udevadm control --reload-rules >/dev/null 2>&1 || true

if [[ -f "$ROLLBACK_DIR/context.env" ]]; then
  # shellcheck disable=SC1090
  source "$ROLLBACK_DIR/context.env"
  if [[ -n "${TARGET_USER:-}" && -n "${TARGET_HOME:-}" && "${OPENDECK_SKIP_KDE_REFRESH:-0}" != "1" ]]; then
    if command -v kbuildsycoca6 >/dev/null 2>&1; then
      if [[ ${EUID:-$(id -u)} -eq 0 && -n "$(command -v sudo 2>/dev/null || true)" ]]; then
        sudo -u "$TARGET_USER" env HOME="$TARGET_HOME" kbuildsycoca6 --noincremental >/dev/null 2>&1 || true
      else
        kbuildsycoca6 --noincremental >/dev/null 2>&1 || true
      fi
    elif command -v kbuildsycoca5 >/dev/null 2>&1; then
      if [[ ${EUID:-$(id -u)} -eq 0 && -n "$(command -v sudo 2>/dev/null || true)" ]]; then
        sudo -u "$TARGET_USER" env HOME="$TARGET_HOME" kbuildsycoca5 --noincremental >/dev/null 2>&1 || true
      else
        kbuildsycoca5 --noincremental >/dev/null 2>&1 || true
      fi
    fi
  fi
fi

say "OPENDECK_V239_ROLLBACK=PASS"
say "OPENDECK_V239_ROLLBACK_STATE=$ROLLBACK_DIR"

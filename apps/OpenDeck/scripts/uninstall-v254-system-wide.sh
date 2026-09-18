#!/usr/bin/env bash
set -Eeuo pipefail

TARGET_VERSION="2.0.54"
OPT_ROOT="${OPENDECK_OPT_ROOT:-/opt/opendeck-plus}"
STATE_ROOT="${OPENDECK_STATE_ROOT:-/var/lib/opendeck-plus}"
ROLLBACK_DIR="$STATE_ROOT/rollback-v$TARGET_VERSION"
VERIFY_FILE="${1:-}"
SELF_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
ROLLBACK_HELPER="$SELF_ROOT/opendeck-rollback"

say(){ printf '%s\n' "$*"; [[ -n "$VERIFY_FILE" ]] && printf '%s\n' "$*" >> "$VERIFY_FILE" || true; }
fail(){ say "OPENDECK_V254_UNINSTALL=FAIL:${1}"; exit "${2:-1}"; }
if [[ ${EUID:-$(id -u)} -ne 0 && "${OPENDECK_ALLOW_NONROOT:-0}" != "1" ]]; then fail "ROOT_REQUIRED" 2; fi
[[ -x "$ROLLBACK_HELPER" ]] || fail "ROLLBACK_HELPER_MISSING:$ROLLBACK_HELPER" 3

"$ROLLBACK_HELPER" "$ROLLBACK_DIR" "$VERIFY_FILE" || fail "ROLLBACK_FAILED" 4
rm -rf -- "$OPT_ROOT/$TARGET_VERSION"
if [[ -L "$OPT_ROOT/current" ]]; then
  target="$(readlink -f "$OPT_ROOT/current" 2>/dev/null || true)"
  [[ "$target" != "$OPT_ROOT/$TARGET_VERSION" ]] || rm -f -- "$OPT_ROOT/current"
fi
say "OPENDECK_V254_UNINSTALL=PASS"
say "OPENDECK_V254_REMOVED=$OPT_ROOT/$TARGET_VERSION"

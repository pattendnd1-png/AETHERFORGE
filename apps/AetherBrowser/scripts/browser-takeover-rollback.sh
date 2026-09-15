#!/usr/bin/env bash
set -euo pipefail
VERSION='2.1.60'
STATE_ROOT="${XDG_STATE_HOME:-$HOME/.local/state}/aetherforge/aether-browser/takeover-v${VERSION}"
APPS_ROOT="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
[[ -d "$STATE_ROOT" && -f "$STATE_ROOT/captured" ]] || { echo 'AETHER_BROWSER_TAKEOVER_ROLLBACK=FAIL:no-state' >&2; exit 2; }
restore_default() {
  local mime=$1 file=$2
  if [[ -s "$file" ]]; then
    xdg-mime default "$(head -n1 "$file")" "$mime"
  fi
}
ASSOCIATIONS=(
  'x-scheme-handler/http'
  'x-scheme-handler/https'
  'text/html'
  'application/xhtml+xml'
  'application/pdf'
  'x-scheme-handler/ftp'
)
for mime in "${ASSOCIATIONS[@]}"; do
  key=$(printf '%s' "$mime" | tr '/:' '__')
  restore_default "$mime" "$STATE_ROOT/defaults/$key.default"
done
printf '%s\n' 'AETHER_BROWSER_TAKEOVER_ROLLBACK_ASSOCIATIONS=PASS'
if [[ -f "$STATE_ROOT/hidden-desktops" ]]; then
  while IFS= read -r base; do
    [[ -n "$base" ]] || continue
    if [[ -f "$STATE_ROOT/backup-$base" ]]; then
      cp -a "$STATE_ROOT/backup-$base" "$APPS_ROOT/$base"
    else
      rm -f "$APPS_ROOT/$base"
    fi
  done < "$STATE_ROOT/hidden-desktops"
fi
rm -f "$STATE_ROOT/active"
printf '%s\n' 'AETHER_BROWSER_TAKEOVER_ROLLBACK=PASS'
if [[ -s "$STATE_ROOT/purged-packages" ]]; then
  printf '%s\n' 'AETHER_BROWSER_TAKEOVER_ROLLBACK_PACKAGES=MANUAL_REINSTALL_REQUIRED'
fi

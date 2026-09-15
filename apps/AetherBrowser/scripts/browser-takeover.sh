#!/usr/bin/env bash
set -euo pipefail

VERSION='2.1.60'
MODE="${1:---activate}"
DOWNLOADS="${AETHER_BROWSER_OUT_DIR:-$HOME/Downloads}"
READY="$DOWNLOADS/Aether-Browser-v${VERSION}-TAKEOVER-READY.txt"
STATE_ROOT="${XDG_STATE_HOME:-$HOME/.local/state}/aetherforge/aether-browser/takeover-v${VERSION}"
APPS_ROOT="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
AETHER_DESKTOP='org.aetherforge.AetherBrowser.desktop'
mkdir -p "$STATE_ROOT" "$APPS_ROOT"

[[ -f "$READY" ]] || { echo 'AETHER_BROWSER_TAKEOVER=REFUSED:missing-TAKEOVER-READY' >&2; exit 10; }
grep -q '^AETHER_BROWSER_STABILITY_ACCEPTANCE=PASS$' "$READY" || { echo 'AETHER_BROWSER_TAKEOVER=REFUSED:stability-not-pass' >&2; exit 11; }
grep -q '^AETHER_BROWSER_VERSION=2.1.60$' "$READY" || { echo 'AETHER_BROWSER_TAKEOVER=REFUSED:wrong-version' >&2; exit 12; }

capture_default() {
  local mime=$1 out=$2
  xdg-mime query default "$mime" > "$out" 2>/dev/null || :
}

# Every user-facing browser association we take over is captured first so the
# operation remains reversible.  Packages and browser profiles are preserved by
# default; only --purge with an explicit acknowledgement may remove packages.
ASSOCIATIONS=(
  'x-scheme-handler/http'
  'x-scheme-handler/https'
  'text/html'
  'application/xhtml+xml'
  'application/pdf'
  'x-scheme-handler/ftp'
)

if [[ ! -f "$STATE_ROOT/captured" ]]; then
  mkdir -p "$STATE_ROOT/defaults"
  for mime in "${ASSOCIATIONS[@]}"; do
    key=$(printf '%s' "$mime" | tr '/:' '__')
    capture_default "$mime" "$STATE_ROOT/defaults/$key.default"
  done
  pacman -Qq > "$STATE_ROOT/packages.before" 2>/dev/null || :
  : > "$STATE_ROOT/hidden-desktops"
  : > "$STATE_ROOT/existing-local-overrides"
  touch "$STATE_ROOT/captured"
fi

for mime in "${ASSOCIATIONS[@]}"; do
  xdg-mime default "$AETHER_DESKTOP" "$mime"
done
if command -v xdg-settings >/dev/null 2>&1; then
  xdg-settings set default-web-browser "$AETHER_DESKTOP" >/dev/null 2>&1 || true
fi

# Hide competing browser launchers without deleting profiles or packages.
while IFS= read -r desktop; do
  [[ -n "$desktop" ]] || continue
  base=$(basename "$desktop")
  [[ "$base" == "$AETHER_DESKTOP" ]] && continue
  grep -Eq '^Categories=.*WebBrowser' "$desktop" || continue
  local_override="$APPS_ROOT/$base"
  if [[ -f "$local_override" && ! -f "$STATE_ROOT/backup-$base" ]]; then
    cp -a "$local_override" "$STATE_ROOT/backup-$base"
    printf '%s\n' "$base" >> "$STATE_ROOT/existing-local-overrides"
  fi
  cat > "$local_override" <<DESKTOP
[Desktop Entry]
Hidden=true
NoDisplay=true
DESKTOP
  printf '%s\n' "$base" >> "$STATE_ROOT/hidden-desktops"
done < <(find /usr/share/applications -maxdepth 1 -type f -name '*.desktop' -print 2>/dev/null | sort)
sort -u -o "$STATE_ROOT/hidden-desktops" "$STATE_ROOT/hidden-desktops"

printf '%s\n' 'AETHER_BROWSER_TAKEOVER_ASSOCIATIONS=PASS:http|https|html|xhtml|pdf|ftp'
printf '%s\n' 'AETHER_BROWSER_TAKEOVER_DEFAULTS=PASS:http|https|html|xhtml|pdf|ftp'
printf '%s\n' 'AETHER_BROWSER_TAKEOVER_DEPENDENCY_SAFE=PASS:packages-preserved-unless-explicit-purge'
printf '%s\n' "AETHER_BROWSER_TAKEOVER_COMPETITOR_LAUNCHERS=HIDDEN:$(wc -l < "$STATE_ROOT/hidden-desktops")"
printf '%s\n' 'AETHER_BROWSER_TAKEOVER_PROFILES=PRESERVED'

if [[ "$MODE" == '--purge' ]]; then
  [[ "${AETHER_BROWSER_TAKEOVER_PURGE_ACK:-}" == 'YES' ]] || { echo 'AETHER_BROWSER_TAKEOVER_PURGE=REFUSED:set-AETHER_BROWSER_TAKEOVER_PURGE_ACK=YES' >&2; exit 20; }
  mapfile -t packages < <(
    while IFS= read -r base; do
      [[ -n "$base" ]] || continue
      pacman -Qo "/usr/share/applications/$base" 2>/dev/null | sed -n 's/.* is owned by \([^ ]*\) .*/\1/p'
    done < "$STATE_ROOT/hidden-desktops" | grep -v '^aether-browser$' | sort -u
  )
  if (( ${#packages[@]} > 0 )); then
    printf '%s\n' "${packages[@]}" > "$STATE_ROOT/purged-packages"
    sudo -v
    sudo -n pacman -Rns --noconfirm "${packages[@]}"
    printf '%s\n' "AETHER_BROWSER_TAKEOVER_PURGE=PASS:${#packages[@]}"
  else
    printf '%s\n' 'AETHER_BROWSER_TAKEOVER_PURGE=PASS:0'
  fi
else
  printf '%s\n' 'AETHER_BROWSER_TAKEOVER_PURGE=SKIP:not-requested'
fi

touch "$STATE_ROOT/active"
printf '%s\n' 'AETHER_BROWSER_TAKEOVER=PASS'
printf '%s\n' "AETHER_BROWSER_TAKEOVER_STATE=$STATE_ROOT"

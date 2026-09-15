#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
STAMP="$(date +%Y%m%d-%H%M%S)"
MODE="${1:-auto}"

if pgrep -x obs >/dev/null 2>&1 || pgrep -x obs-studio >/dev/null 2>&1; then
  echo "OBS is running. Close OBS completely before installing AetherForge." >&2
  exit 2
fi

NATIVE_BASIC="${XDG_CONFIG_HOME:-$HOME/.config}/obs-studio/basic"
FLATPAK_BASIC="$HOME/.var/app/com.obsproject.Studio/config/obs-studio/basic"
NATIVE_MEDIA="${XDG_DATA_HOME:-$HOME/.local/share}/aetherforge-streaming/v3.0.5"
FLATPAK_MEDIA="$HOME/.var/app/com.obsproject.Studio/data/aetherforge-streaming/v3.0.5"

have_native=0
have_flatpak=0
[ -d "${XDG_CONFIG_HOME:-$HOME/.config}/obs-studio" ] && have_native=1
[ -d "$HOME/.var/app/com.obsproject.Studio" ] && have_flatpak=1
command -v obs >/dev/null 2>&1 && have_native=1
if command -v flatpak >/dev/null 2>&1 && flatpak info com.obsproject.Studio >/dev/null 2>&1; then have_flatpak=1; fi

targets=()
case "$MODE" in
  --native|native) targets+=("native") ;;
  --flatpak|flatpak) targets+=("flatpak") ;;
  --all|all) targets+=("native" "flatpak") ;;
  auto)
    [ "$have_native" -eq 1 ] && targets+=("native")
    [ "$have_flatpak" -eq 1 ] && targets+=("flatpak")
    if [ "${#targets[@]}" -eq 0 ]; then targets+=("native"); fi
    ;;
  *) echo "Usage: bash scripts/install.sh [auto|--native|--flatpak|--all]" >&2; exit 64 ;;
esac

render_scene() {
  local src="$1" dst="$2" media_root="$3"
  python3 - "$src" "$dst" "$media_root" <<'PYSCENE'
import json, pathlib, sys
src,dst,media=sys.argv[1:4]
data=json.loads(pathlib.Path(src).read_text())
for s in data.get('sources',[]):
    st=s.get('settings',{})
    if isinstance(st,dict) and isinstance(st.get('local_file'),str):
        st['local_file']=st['local_file'].replace('__MEDIA_ROOT__', media)
pathlib.Path(dst).write_text(json.dumps(data, indent=2, ensure_ascii=False)+'\n')
PYSCENE
}

install_one() {
  local kind="$1" basic media_root
  if [ "$kind" = native ]; then
    basic="$NATIVE_BASIC"; media_root="$NATIVE_MEDIA"
  else
    basic="$FLATPAK_BASIC"; media_root="$FLATPAK_MEDIA"
  fi
  local scenes="$basic/scenes" profiles="$basic/profiles" backup="$basic/aetherforge-backups/$STAMP"
  mkdir -p "$scenes" "$profiles" "$media_root" "$backup/scenes" "$backup/profiles"

  rm -rf "$media_root/media" "$media_root/audio"
  cp -a "$ROOT/media" "$media_root/media"
  cp -a "$ROOT/audio" "$media_root/audio"

  for res in 1920x1080 2560x1440; do
    local src="$ROOT/obs/AetherForge-v3.0.5-$res.json"
    local dst="$scenes/AetherForge-v3.0.5-$res.json"
    [ -f "$dst" ] && cp -a "$dst" "$backup/scenes/" || true
    render_scene "$src" "$dst" "$media_root"
  done

  for dirname in AetherForge-v3.0.5-1080p AetherForge-v3.0.5-1440p; do
    local src="$ROOT/obs/profiles/$dirname" dst="$profiles/$dirname"
    [ -d "$dst" ] && cp -a "$dst" "$backup/profiles/" || true
    rm -rf "$dst"
    cp -a "$src" "$dst"
  done

  echo "Installed $kind OBS package:"
  echo "  Scenes : $scenes"
  echo "  Profiles: $profiles"
  echo "  Media  : $media_root"
  echo "  Backup : $backup"
}

for t in "${targets[@]}"; do install_one "$t"; done

echo
echo "AetherForge v3.0.5 installation complete."
echo "Run scripts/diagnose.sh to verify the installed paths before opening OBS."
echo "Use scripts/launch-1080p.sh or scripts/launch-1440p.sh to open the exact profile + scene collection pair."

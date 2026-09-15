#!/usr/bin/env bash
set -euo pipefail
native_basic="${XDG_CONFIG_HOME:-$HOME/.config}/obs-studio/basic"
flatpak_basic="$HOME/.var/app/com.obsproject.Studio/config/obs-studio/basic"
status=0
check_target() {
  local label="$1" basic="$2"
  [ -d "$basic" ] || return 0
  echo "== $label =="
  echo "OBS basic path: $basic"
  for res in 1920x1080 2560x1440; do
    f="$basic/scenes/AetherForge-v3.0.5-$res.json"
    if [ ! -f "$f" ]; then echo "MISSING scene collection: $f"; status=1; continue; fi
    python3 - "$f" <<'PYDIAG' || status=1
import json, pathlib, sys
p=pathlib.Path(sys.argv[1]); d=json.loads(p.read_text()); missing=[]
for s in d.get('sources',[]):
    f=s.get('settings',{}).get('local_file')
    if f and not pathlib.Path(f).exists(): missing.append(f)
print(f"{p.name}: {len(d.get('sources',[]))} sources; missing media={len(missing)}")
for f in missing: print('  MISSING:',f)
raise SystemExit(1 if missing else 0)
PYDIAG
  done
}
check_target native "$native_basic"
check_target flatpak "$flatpak_basic"
if command -v obs >/dev/null 2>&1; then obs --version || true; fi
if command -v flatpak >/dev/null 2>&1 && flatpak info com.obsproject.Studio >/dev/null 2>&1; then flatpak info com.obsproject.Studio | sed -n '1,12p'; fi
exit "$status"

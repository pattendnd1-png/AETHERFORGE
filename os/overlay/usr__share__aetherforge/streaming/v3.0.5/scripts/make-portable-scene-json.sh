#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MEDIA_ROOT="${1:-${XDG_DATA_HOME:-$HOME/.local/share}/aetherforge-streaming/v3.0.5}"
OUT="$ROOT/obs/ready"
mkdir -p "$OUT"
for src in "$ROOT"/obs/AetherForge-v3.0.5-*.json; do
  dst="$OUT/$(basename "$src")"
  python3 - "$src" "$dst" "$MEDIA_ROOT" <<'PYSCENE'
import json,pathlib,sys
src,dst,media=sys.argv[1:4]
d=json.loads(pathlib.Path(src).read_text())
for s in d.get("sources",[]):
    st=s.get("settings",{})
    if isinstance(st.get("local_file"),str): st["local_file"]=st["local_file"].replace("__MEDIA_ROOT__",media)
pathlib.Path(dst).write_text(json.dumps(d,indent=2,ensure_ascii=False)+"\n")
PYSCENE
done
echo "Rendered scene collections to $OUT"

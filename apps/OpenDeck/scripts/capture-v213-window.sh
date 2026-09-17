#!/usr/bin/env bash
set -euo pipefail
OUT="${1:?usage: capture-v213-window.sh OUTPUT.png}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
command -v spectacle >/dev/null 2>&1 || { echo 'OPENDECK_V213_CAPTURE=FAIL:SPECTACLE_MISSING'; exit 2; }
RAW="${OUT%.png}.raw.png"
rm -f "$RAW" "$OUT"
spectacle -b -n -a -o "$RAW"
[[ -s "$RAW" ]] || { echo 'OPENDECK_V213_CAPTURE=FAIL:EMPTY'; exit 3; }
python3 "$ROOT/scripts/v213-normalize-capture.py" "$RAW" "$OUT"
rm -f "$RAW"
[[ -s "$OUT" ]] || { echo 'OPENDECK_V213_CAPTURE=FAIL:NORMALIZE_EMPTY'; exit 4; }
echo "OPENDECK_V213_CAPTURE=PASS:$OUT"

#!/usr/bin/env bash
set -euo pipefail
file="$(cd "$(dirname "$0")/.." && pwd)/crates/forgehx-cli/src/main.rs"
grep -q 'enum MicDspCommand' "$file"
grep -q 'Command::MicDspGet' "$file"
grep -q 'Command::MicDspSave' "$file"
grep -q 'Command::MicDspApply' "$file"
grep -q 'Command::MicDspBypass' "$file"
grep -q 'Reply::MicDspState' "$file"
echo 'ForgeHX microphone CLI invariants passed.'

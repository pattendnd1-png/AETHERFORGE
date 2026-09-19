#!/usr/bin/env bash
set -euo pipefail
ROOT="${1:-$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)}"
FILE="$ROOT/src/private_audio.rs"
fail(){ echo "AETHERFORGE_BEACN_V0_1_18_PRIVATE_AUDIO_CLIPPY=FAIL:$1"; exit 1; }
[[ -f "$FILE" ]] || fail missing_private_audio
if grep -Fq 'Err(error) if stop.load(Ordering::Acquire) => break,' "$FILE"; then
  fail unused_guard_error_binding
fi
if grep -Fq 'input_bytes.chunks_exact(4)' "$FILE"; then
  fail chunks_exact_regression
fi
grep -Fq 'Err(_) if stop.load(Ordering::Acquire) => break,' "$FILE" || fail wildcard_stop_guard_missing
grep -Fq 'input_bytes.as_chunks::<4>().0.iter().enumerate()' "$FILE" || fail fixed_width_chunking_missing
echo 'AETHERFORGE_BEACN_V0_1_18_PRIVATE_AUDIO_CLIPPY=PASS'

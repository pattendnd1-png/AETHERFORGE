#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
cd "$ROOT"

# Rust 1.98 Clippy regressions found by the v0.4.0 host gate.
# The ColdPack restore loop must not add an extra borrow to an already-borrowed PathBuf.
if grep -Fq 'staging.join(&relative)' src/coldstore.rs; then
  echo 'FORGECLEAN_V0_4_1_CLIPPY=FAIL:NEEDLESS_BORROW' >&2
  exit 1
fi
# Rust 1.98 prefers slice::as_chunks for fixed-size chunks.
if grep -Fq 'bytes.chunks_exact(2)' src/coldstore.rs; then
  echo 'FORGECLEAN_V0_4_1_CLIPPY=FAIL:CHUNKS_EXACT' >&2
  exit 1
fi
printf '%s\n' 'FORGECLEAN_V0_4_1_CLIPPY=PASS'

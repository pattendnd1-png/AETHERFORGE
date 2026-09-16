#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
grep -Fq 'if let Some(asset) = fields.get(4)' src/orbital.rs
grep -Fq '&& !asset.is_empty()' src/orbital.rs
grep -Fq '&& !names.lines().any(|name| name == *asset)' src/orbital.rs
if grep -Fq 'if let Some(asset) = fields.get(4) {' src/orbital.rs; then
  echo 'old collapsible nested-if still present' >&2
  exit 1
fi
echo 'FORGECLEAN_V1_0_11_CLIPPY_REGRESSION=PASS'

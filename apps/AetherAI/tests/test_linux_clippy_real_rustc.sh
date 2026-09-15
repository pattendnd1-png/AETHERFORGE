#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
cd "$ROOT"
./scripts/aetherai-rust clippy --offline -p xtask --all-targets -- -D warnings
printf '%s\n' 'AETHERAI_LINUX_CLIPPY_REAL_RUSTC_TEST=PASS'

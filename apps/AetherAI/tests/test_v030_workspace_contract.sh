#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

grep -q '^version = "0.3.0"' Cargo.toml
grep -q '"crates/aether-index"' Cargo.toml
grep -q '"crates/aether-retrieval"' Cargo.toml
test -f crates/aether-index/src/lib.rs
test -f crates/aether-retrieval/src/lib.rs
! grep -R --line-number 'Work mode\|WorkMode\|aetherai-work' apps crates --exclude-dir=target

echo AETHERAI_V030_WORKSPACE_CONTRACT=PASS

#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
grep -Fq '[workspace]' "$ROOT/Cargo.toml"
grep -Fq 'a.parse::<u32>()' "$ROOT/tests/package.rs"
grep -Fq 'b.parse::<u32>()' "$ROOT/tests/package.rs"
grep -Fq 'FORGECLEAN_PACKAGE_TEST cargo test --test package' "$ROOT/build-and-verify.sh"

#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

grep -q '^version = "10.0.16"$' Cargo.toml || { echo 'workspace version 10.0.16 missing' >&2; exit 1; }
grep -q '^pkgver=10.0.16$' PKGBUILD || { echo 'PKGBUILD pkgver 10.0.16 missing' >&2; exit 1; }
grep -q '^pkgrel=1$' PKGBUILD || { echo 'PKGBUILD pkgrel 1 missing' >&2; exit 1; }
grep -q 'python scripts/test-10.0.14-direct-target-id.py' scripts/test-10.0.14-base-release.sh || { echo '10.0.14 target-id regression not preserved' >&2; exit 1; }
grep -q 'pub const IPC_PROTOCOL_VERSION: u32 = 11;' crates/forgehx-core/src/lib.rs || { echo 'IPC v11 missing' >&2; exit 1; }
echo 'ForgeHX 10.0.16 recovery release identity invariants passed.'

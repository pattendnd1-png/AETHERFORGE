#!/usr/bin/env bash
set -euo pipefail
root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd "$root"

for command in cargo rustc; do
  if ! command -v "$command" >/dev/null 2>&1; then
    echo "ERROR: $command is required. Install the Arch rust/cargo toolchain first." >&2
    exit 2
  fi
done

echo "== OpenSanctuary v0.4.2 =="
rustc --version
cargo --version

echo "== Normalize Rust formatting =="
cargo fmt --all

echo "== Run strict verifier =="
./scripts/verify.sh

echo
echo "Build verified. Launch with:"
echo "  $root/target/release/opensanctuary-launcher"

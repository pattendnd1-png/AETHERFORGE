#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

command -v cargo >/dev/null 2>&1 || {
  printf '%s\n' 'ERROR: cargo is required. On Arch: sudo pacman -S --needed rust' >&2
  exit 1
}

cargo test --workspace
cargo build --release --workspace
printf '\nBuilt binaries:\n'
printf '  %s\n' \
  "${ROOT}/target/release/reforge-logitech" \
  "${ROOT}/target/release/reforge-logitechctl" \
  "${ROOT}/target/release/reforge-logitechd"

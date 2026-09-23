#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
MANIFEST="$ROOT/Cargo.toml"
grep -qx '\[workspace\]' "$MANIFEST" || {
  echo "FORGECLEAN_WORKSPACE_ISOLATION_STATIC=FAIL:MISSING_ROOT_WORKSPACE"
  exit 1
}
count="$(grep -xc '\[workspace\]' "$MANIFEST")"
[[ "$count" -eq 1 ]] || {
  echo "FORGECLEAN_WORKSPACE_ISOLATION_STATIC=FAIL:DUPLICATE_ROOT_WORKSPACE"
  exit 1
}
echo "FORGECLEAN_WORKSPACE_ISOLATION_STATIC=PASS"

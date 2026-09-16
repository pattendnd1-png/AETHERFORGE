#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
cd "$ROOT"
grep -Eq '^version = "[0-9]+\.[0-9]+\.[0-9]+"$' Cargo.toml
if ! grep -Eq '^const VERSION: &str = "[0-9]+\.[0-9]+\.[0-9]+";$' src/main.rs \
   && ! grep -Fxq 'const VERSION: &str = env!("CARGO_PKG_VERSION");' src/main.rs; then
  echo 'FORGECLEAN_V0_3_0_VERSION_AUTHORITY=FAIL' >&2
  exit 1
fi
for mod in organizer registry coldstore; do
  [[ -f "src/${mod}.rs" ]]
  grep -Fq "pub mod ${mod};" src/lib.rs
done
for cmd in '"organize-once"' '"watch"' '"activate"' '"resolve-project"' '"build"' '"archive"' '"restore"'; do
  grep -Fq "$cmd" src/main.rs
done
grep -Fq 'ForgeClean/Projects' README.md
grep -Fq 'fcold.tar.zst' README.md
grep -Fq 'Active source trees are never cold-compressed' README.md
grep -Fq 'archive_to_cold' src/coldstore.rs
grep -Fq 'restore_cold_archive' src/coldstore.rs
grep -Fq 'AETHER_GUARD=BLOCKED active project trees are never cold-compressed' src/main.rs
[[ -f forgeclean-organizer.service.in ]]
grep -Fq 'ExecStart=%h/.local/bin/forgeclean watch' forgeclean-organizer.service.in
grep -Fq 'FORGECLEAN_V0_3_0_REGRESSION' build-and-verify.sh
echo FORGECLEAN_V0_3_0_REGRESSION=PASS

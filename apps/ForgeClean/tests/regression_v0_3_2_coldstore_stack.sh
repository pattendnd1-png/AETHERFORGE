#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
COLD="$ROOT/src/coldstore.rs"
SECTION="$(mktemp)"
trap 'rm -f -- "$SECTION"' EXIT
awk '
  /^fn hash_tree\(/ { in_fn=1 }
  in_fn { print }
  in_fn && /^fn hash_file\(/ { exit }
' "$COLD" > "$SECTION"
grep -Fq 'let mut buffer = vec![0u8; HASH_BUFFER_BYTES];' "$SECTION"
grep -Fq 'hash_tree_inner(path, Path::new(root_name), &mut hasher, &mut buffer)?;' "$SECTION"
grep -Fq 'buffer: &mut [u8]' "$SECTION"
if grep -Fq 'let mut buffer = [0u8; 1024 * 1024];' "$SECTION"; then
  echo 'FORGECLEAN_V0_3_2_COLDSTORE_STACK=FAIL:LARGE_RECURSIVE_STACK_BUFFER'
  exit 1
fi
echo 'FORGECLEAN_V0_3_2_COLDSTORE_STACK=PASS'

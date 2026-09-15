#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
cd "$ROOT"
grep -Fq 'deeply_nested_tree_archives_and_restores_without_stack_exhaustion' tests/coldstore.rs
grep -Fq 'FORGECLEAN_COLDSTORE_DEEP_TREE_TEST' build-and-verify.sh
grep -Fq 'FORGECLEAN_V0_3_2_COLDSTORE_STACK' build-and-verify.sh
echo FORGECLEAN_V0_3_2_REGRESSION=PASS

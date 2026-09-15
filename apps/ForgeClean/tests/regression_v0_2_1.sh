#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
grep -Fq 'let parent = current.parent_name.as_deref()?;' "$ROOT/src/storage.rs"
grep -Fq 'let next = by_name.get(parent).copied()?;' "$ROOT/src/storage.rs"
grep -Fq 'FORGECLEAN_CLIPPY_QUESTION_MARK_REGRESSION' "$ROOT/build-and-verify.sh"
echo FORGECLEAN_V0_2_1_REGRESSION=PASS

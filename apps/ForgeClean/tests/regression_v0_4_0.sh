#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
cd "$ROOT"
grep -Fq 'pub const COLDPACK_SUFFIX: &str = ".fcoldpack"' src/coldstore.rs
grep -Fq 'pub fn archive_to_coldpack' src/coldstore.rs
grep -Fq 'pub fn restore_coldpack_archive' src/coldstore.rs
grep -Fq 'pub fn coldpack_store' src/organizer.rs
grep -Fq '.coldpack-store' src/organizer.rs
grep -Fq 'FORGECLEAN_COLDSTORE_FORMAT=FCOLDPACK_CDC_DEDUP_ZSTD_SHA256' build-and-verify.sh
grep -Fq 'FORGECLEAN_V0_4_0_COLDPACK_DEDUP' build-and-verify.sh
grep -Fq 'systemctl --user restart forgeclean-organizer.service' install-local.sh
printf '%s\n' 'FORGECLEAN_V0_4_0_REGRESSION=PASS'

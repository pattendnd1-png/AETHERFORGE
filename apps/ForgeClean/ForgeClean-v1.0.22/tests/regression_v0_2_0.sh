#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
grep -Fq 'PACMAN-BATCH.txt' "$ROOT/src/system_scan.rs"
grep -Fq '"storage-status"' "$ROOT/src/main.rs"
grep -Fq '"auto"' "$ROOT/src/main.rs"
grep -Fq 'AUTO_CLEAN' "$ROOT/src/main.rs"
grep -Fq 'AUTO_OFFLOAD' "$ROOT/src/main.rs"
grep -Fq 'pub mod storage;' "$ROOT/src/lib.rs"
grep -Fq 'pub mod offload;' "$ROOT/src/lib.rs"
grep -Fq 'sha2' "$ROOT/Cargo.toml"
echo FORGECLEAN_V0_2_0_REGRESSION=PASS
grep -Fq 'detect_external_drives(&batch.root)?' "$ROOT/src/main.rs"
grep -Fq 'prepare_destination_under_mount(&drive.mount_point)?' "$ROOT/src/main.rs"
grep -Fq 'source changed during offload; source preserved' "$ROOT/src/offload.rs"
grep -Fq 'offload destination escaped external mount' "$ROOT/src/offload.rs"

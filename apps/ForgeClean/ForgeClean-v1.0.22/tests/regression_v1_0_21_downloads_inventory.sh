#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
cd "$ROOT"
grep -Eq '^[[:space:]]*version[[:space:]]*=[[:space:]]*"[0-9]+\.[0-9]+\.[0-9]+"' Cargo.toml
grep -Fq 'pub mod downloads_inventory;' src/lib.rs
grep -Fq 'scan_downloads_inventory' src/downloads_inventory.rs
grep -Fq 'fs::symlink_metadata' src/downloads_inventory.rs
grep -Fq 'meta.blocks().saturating_mul(512)' src/downloads_inventory.rs
grep -Fq 'GENERATED' src/downloads_inventory.rs
grep -Fq 'DownloadsInventoryReport' src/gui_state.rs
grep -Fq 'scan_downloads_inventory_summary(downloads)?' src/gui_state.rs
grep -Fq 'Downloads — General Files' src/gui.rs
grep -Fq 'Downloads — Project Files' src/gui.rs
grep -Fq 'protected from automatic cleanup' src/gui.rs
grep -Fq '"downloads-inventory" => downloads_inventory_command(&home, &args)?' src/bin/forgeclean-system.rs
grep -Fq 'FORGECLEAN_DOWNLOADS_INVENTORY_TEST' build-and-verify.sh
grep -Fq 'FORGECLEAN_DOWNLOADS_INVENTORY_E2E' build-and-verify.sh
printf 'FORGECLEAN_V1_0_21_DOWNLOADS_INVENTORY=PASS\n'

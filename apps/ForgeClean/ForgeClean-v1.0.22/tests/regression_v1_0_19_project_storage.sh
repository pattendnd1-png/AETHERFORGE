#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
cd "$ROOT"
# Historical feature regression: forward releases must remain eligible.
grep -Eq '^[[:space:]]*version[[:space:]]*=[[:space:]]*"[0-9]+\.[0-9]+\.[0-9]+"' Cargo.toml
grep -Fq 'pub mod project_storage;' src/lib.rs
grep -Fq 'scan_project_download_storage' src/project_storage.rs
grep -Fq 'MetadataExt' src/project_storage.rs
grep -Fq 'meta.blocks().saturating_mul(512)' src/project_storage.rs
grep -Fq '"target"' src/project_storage.rs
grep -Fq '"build"' src/project_storage.rs
grep -Fq '"node_modules"' src/project_storage.rs
grep -Fq 'build_identity_from_entry' src/project_storage.rs
grep -Fq 'ProjectStorageReport' src/gui_state.rs
grep -Fq 'scan_project_download_storage(downloads)?' src/gui_state.rs
grep -Fq 'Project download footprint' src/gui.rs
grep -Fq 'filesystem-allocated blocks' src/gui.rs
grep -Fq '"project-storage" => project_storage_command(&home)?' src/bin/forgeclean-system.rs
grep -Fq 'FORGECLEAN_PROJECT_STORAGE_TEST' build-and-verify.sh
grep -Fq 'FORGECLEAN_PROJECT_STORAGE_E2E' build-and-verify.sh
printf 'FORGECLEAN_V1_0_19_PROJECT_STORAGE=PASS\n'

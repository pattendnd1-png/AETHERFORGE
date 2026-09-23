#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
cd "$ROOT"
grep -Eq '^version[[:space:]]*=[[:space:]]*"1\.0\.22"' Cargo.toml
grep -Fq 'pub mod project_routing;' src/lib.rs
grep -Fq 'scan_downloads_inventory_summary' src/downloads_inventory.rs
grep -Fq 'FORGECLEAN_SCAN_THREADS' src/downloads_inventory.rs
grep -Fq 'ProjectIndex::discover' src/downloads_inventory.rs
grep -Fq 'family_for_artifact_name' src/project_routing.rs
grep -Fq 'strip_action_prefixes' src/project_routing.rs
grep -Fq 'GENERAL_POLICY=LEAVE_IN_DOWNLOADS' src/bin/forgeclean-project-router.rs
grep -Fq 'canonical_project_root' src/bin/forgeclean-project-router.rs
grep -Fq 'downloads-inventory --details' build-and-verify.sh
grep -Fq 'scan_downloads_inventory_summary(downloads)?' src/gui_state.rs
printf 'FORGECLEAN_V1_0_22_NAMED_PROJECTS_FAST_SCAN=PASS\n'

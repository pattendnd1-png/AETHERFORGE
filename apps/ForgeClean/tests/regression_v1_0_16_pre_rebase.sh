#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
cd "$ROOT"
grep -Eq '^version[[:space:]]*=[[:space:]]*"1\.0\.16"' Cargo.toml
grep -Fq 'pub mod pre_rebase;' src/lib.rs
grep -Fq 'GARUDA_REBASE_CUTOFF_UNIX_SECS: i64 = 1_787_036_400' src/pre_rebase.rs
grep -Fq 'STORAGE_PRESSURE_PERCENT: u8 = 70' src/pre_rebase.rs
grep -Fq 'STORAGE_CRITICAL_PERCENT: u8 = 85' src/pre_rebase.rs
grep -Fq 'fs::remove_file(&candidate.path)' src/pre_rebase.rs
grep -Fq 'parents_are_symlink_free' src/pre_rebase.rs
grep -Fq 'is_vcs_checkout_dir' src/pre_rebase.rs
grep -Fq 'is_current_release_source' src/pre_rebase.rs
grep -Fq '"pre-rebase" => pre_rebase_command' src/bin/forgeclean-system.rs
grep -Fq 'pre-rebase apply --yes --reason install' install-local.sh
grep -Fq 'forgeclean-pre-rebase.timer' install-local.sh
grep -Fq 'FORGECLEAN_PRE_REBASE_TEST' build-and-verify.sh
grep -Fq 'FORGECLEAN_PRE_REBASE_E2E' build-and-verify.sh
grep -Fq 'pre-rebase auto --yes' systemd/forgeclean-pre-rebase.service.in
grep -Fq 'OnUnitActiveSec=1h' systemd/forgeclean-pre-rebase.timer.in
printf 'FORGECLEAN_V1_0_16_PRE_REBASE=PASS\n'

#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
fail(){ echo "AETHER_BROWSER_SERVO_OPTS_CONTRACT=FAIL:$1"; exit 1; }
FILE='crates/aether-engine-servo/src/live.rs'
grep -qF 'let servo_opts = Opts {' "$FILE" || fail direct-initializer-missing
grep -qF 'config_dir: Some(profile_storage.root().to_path_buf()),' "$FILE" || fail config-dir-initializer-missing
grep -qF 'temporary_storage: profile_storage.is_ephemeral(),' "$FILE" || fail temporary-storage-initializer-missing
! grep -qF 'let mut servo_opts = Opts::default();' "$FILE" || fail field-reassign-default-pattern-present
! grep -qF 'servo_opts.config_dir =' "$FILE" || fail config-dir-reassignment-present
! grep -qF 'servo_opts.temporary_storage =' "$FILE" || fail temporary-storage-reassignment-present
echo 'AETHER_BROWSER_SERVO_OPTS_CONTRACT=PASS'

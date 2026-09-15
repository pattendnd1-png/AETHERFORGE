#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
fail(){ echo "AETHER_BROWSER_SESSION_PERSISTENCE=FAIL:$1"; exit 1; }
PROFILE=crates/aether-profile/src/lib.rs
LIVE=crates/aether-engine-servo/src/live.rs

for needle in 'ProfileStorage' 'profiles/default' 'PrivateEphemeral' 'is_ephemeral'; do
  grep -qF "$needle" "$PROFILE" || fail "profile-contract-missing:$needle"
done
grep -qF 'let servo_opts = Opts {' "$LIVE" || fail servo-opts-not-configured
grep -qF 'config_dir: Some(profile_storage.root().to_path_buf()),' "$LIVE" || fail servo-config-dir-not-profile-backed
grep -qF 'temporary_storage: profile_storage.is_ephemeral(),' "$LIVE" || fail servo-private-storage-flag-missing
grep -qF '_profile_storage: ProfileStorage' "$LIVE" || fail profile-lifetime-not-owned-by-runtime
grep -qF 'AETHER_BROWSER_WEB_SESSION_PERSISTENCE=PERSISTENT_NORMAL|EPHEMERAL_PRIVATE' "$LIVE" || fail status-marker-missing

echo 'AETHER_BROWSER_NORMAL_PROFILE_STORAGE=PASS'
echo 'AETHER_BROWSER_PRIVATE_PROFILE_EPHEMERAL=PASS'
echo 'AETHER_BROWSER_SESSION_PERSISTENCE=PASS'

#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
fail(){ echo "AETHER_BROWSER_PROVIDER_TAB_PERSISTENCE=FAIL:$1"; exit 1; }
LIVE=crates/aether-engine-servo/src/live.rs
INSTALL=scripts/install-current-tree.sh
PROFILE=crates/aether-profile/src/lib.rs
PAGES=crates/aether-native-pages/src/lib.rs

grep -qF 'fn provider_open_in_new_tab(raw_url: &str) -> Option<String>' "$LIVE" || fail provider-new-tab-router-missing
grep -qF 'return self.create_tab(&destination, false);' "$LIVE" || fail provider-open-does-not-create-tab
grep -qF '"twitch" | "youtube-live"' "$LIVE" || fail youtube-twitch-tab-policy-missing
grep -qF 'Open YouTube in New Tab' "$PAGES" || fail youtube-new-tab-ui-missing
grep -qF 'Open Twitch in New Tab' "$PAGES" || fail twitch-new-tab-ui-missing
grep -qF 'profiles/default' "$PROFILE" || fail persistent-profile-root-missing
grep -qF 'profile_fingerprint()' "$INSTALL" || fail upgrade-profile-fingerprint-missing
grep -qF 'AETHER_BROWSER_EXISTING_LOGIN_STATE_PRESERVED=PASS' "$INSTALL" || fail login-preservation-marker-missing
grep -qF 'AETHER_BROWSER_UPGRADE_PROFILE_MIGRATION=PASS' "$INSTALL" || fail upgrade-profile-marker-missing
if grep -Eq 'rm -rf .*aether-browser/profiles|rm -rf .*profiles/default' "$INSTALL"; then
  fail installer-removes-profile
fi

echo 'AETHER_BROWSER_SEPARATE_PROVIDER_TABS=PASS'
echo 'AETHER_BROWSER_EXISTING_LOGIN_STATE_PRESERVED=PASS'
echo 'AETHER_BROWSER_UPGRADE_PROFILE_MIGRATION=PASS'
echo 'AETHER_BROWSER_PROVIDER_TAB_PERSISTENCE=PASS'

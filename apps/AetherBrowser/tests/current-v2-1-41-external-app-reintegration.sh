#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
COMPAT="$ROOT/crates/aether-compat/src/lib.rs"
ENGINE="$ROOT/crates/aether-engine-servo/src/live.rs"

fail() { echo "AETHER_BROWSER_V2_1_47_EXTERNAL_APP_REINTEGRATION=FAIL:$1"; exit 1; }

for path in "$COMPAT" "$ENGINE"; do
  [[ -f "$path" ]] || fail "missing:${path#$ROOT/}"
done

grep -q 'pub fn adopt_surfaces' "$COMPAT" || fail 'x11-adopt-surfaces-missing'
grep -q '/proc/{pid}/exe' "$COMPAT" || fail 'orphan-process-identity-missing'
grep -q 'reparent_window(window, self.parent' "$COMPAT" || fail 'orphan-reparent-missing'

grep -q 'fn known_external_app_targets' "$ENGINE" || fail 'known-integrated-app-registry-missing'
grep -q 'fn adopt_orphaned_external_apps' "$ENGINE" || fail 'startup-reintegration-missing'
grep -q 'AETHER_BROWSER_EXTERNAL_APP_RESTART_REINTEGRATION=PASS' "$ENGINE" || fail 'restart-reintegration-pass-marker-missing'
grep -q 'AETHER_BROWSER_EXTERNAL_APP_RESTART_REINTEGRATION=NONE' "$ENGINE" || fail 'restart-reintegration-none-marker-missing'
grep -q 'adopt_orphaned_external_apps' "$ENGINE" || fail 'startup-reintegration-not-called'

grep -q 'AETHER_BROWSER_EXTERNAL_APP_RESTART_PRESERVE=PASS' "$ENGINE" || fail 'detached-window-not-preserved-for-next-run'
grep -q 'surface.attached()' "$ENGINE" || fail 'shutdown-does-not-distinguish-detached-surface'

# Restored tabs must use canonical embedded URLs. Detached mode is transient only.
grep -q 'canonical_external_app_url' "$ENGINE" || fail 'canonical-embedded-url-helper-missing'
if grep -Eq 'metadata.*mode=detached|TabMetadata::new\([^;]*mode=detached' "$ENGINE"; then
  fail 'detached-mode-persisted-in-tab-metadata'
fi

echo 'AETHER_BROWSER_V2_1_47_EXTERNAL_APP_REINTEGRATION=PASS'

#!/usr/bin/env bash
set -euo pipefail
fail(){ echo "AETHER_BROWSER_V2_1_34_OBS_BACKEND_SUPERVISOR=FAIL:$1" >&2; exit 1; }
OBS='crates/aether-stream-studio/src/obs.rs'
NATIVE='crates/aether-native-pages/src/lib.rs'
INSTALL='scripts/install-current-tree.sh'

grep -qF 'ObsBackendSupervisor' "$OBS" || fail supervisor-type-missing
grep -qF 'ensure_running' "$OBS" || fail ensure-running-missing
grep -qF '"--minimize-to-tray"' "$OBS" || fail minimize-to-tray-missing
grep -qF 'AETHER_BROWSER_OBS_BACKEND=STARTED' "$OBS" || fail startup-marker-missing
grep -qF 'query_obs_status_with_autostart' "$OBS" || fail autostart-query-missing
grep -qF 'query_obs_status_with_autostart' "$NATIVE" || fail studio-ui-not-using-autostart
grep -q "'obs-studio'" "$INSTALL" || fail obs-studio-package-dependency-missing

echo 'AETHER_BROWSER_V2_1_34_OBS_BACKEND_SUPERVISOR=PASS'

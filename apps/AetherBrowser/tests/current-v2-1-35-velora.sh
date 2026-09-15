#!/usr/bin/env bash
set -euo pipefail
fail(){ echo "AETHER_BROWSER_V2_1_35_VELORA=FAIL:$1"; exit 1; }
PROVIDERS=crates/aether-stream-providers/src/lib.rs
LIVE=crates/aether-engine-servo/src/live.rs
OBS=crates/aether-stream-studio/tests/obs_protocol.rs
grep -qF 'id: "velora".into()' "$PROVIDERS" || fail provider-missing
grep -qF 'name: "Velora.tv".into()' "$PROVIDERS" || fail provider-name-missing
grep -qF 'kind: ProviderKind::Velora' "$PROVIDERS" || fail provider-kind-missing
grep -qF 'auth: WebSession' "$PROVIDERS" || fail web-session-auth-missing
grep -qF '"velora" => Some("https://velora.tv/".to_owned())' "$LIVE" || fail route-missing
grep -qF '"https://velora.tv/",' "$LIVE" || fail cookie-migration-missing
if grep -qF '"GetTransitionList"' "$OBS"; then fail stale-obs-transition-request; fi
grep -qF '"GetSceneTransitionList"' "$OBS" || fail official-obs-transition-request-missing
echo 'AETHER_BROWSER_V2_1_35_VELORA=PASS'

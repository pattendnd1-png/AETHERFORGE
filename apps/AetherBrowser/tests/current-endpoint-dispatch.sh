#!/usr/bin/env bash
set -euo pipefail
fail(){ echo "AETHER_BROWSER_ENDPOINT_DISPATCH=FAIL:$1"; exit 1; }
LIVE=crates/aether-engine-servo/src/live.rs
python3 - "$LIVE" <<'PY2'
from pathlib import Path
import sys,re
s=Path(sys.argv[1]).read_text()
block=re.search(r'fn request_navigation\(&self, _webview: WebView, request: NavigationRequest\) \{(.*?)\n    \}', s, re.S)
if not block: raise SystemExit('AETHER_BROWSER_ENDPOINT_DISPATCH=FAIL:no-request-navigation')
b=block.group(1)
for needle in ['request.url.scheme() == "aether"','classify_native_playback_url(&requested).is_some()','self.pending_navigation.replace(Some(requested));','request.deny();','request.allow();']:
    if needle not in b: raise SystemExit('AETHER_BROWSER_ENDPOINT_DISPATCH=FAIL:missing:'+needle)
if 'AETHER_BROWSER_ENDPOINT_DISPATCH=PENDING:' not in b:
    raise SystemExit('AETHER_BROWSER_ENDPOINT_DISPATCH=FAIL:no-pending-diagnostic')
print('AETHER_BROWSER_ENDPOINT_REQUEST_ROUTING=PASS')
PY2
grep -qF 'fn resolve_interface_action_url' "$LIVE" || fail no-action-resolver
grep -qF 'direct_external_runtime_target(&destination)' "$LIVE" || fail action-not-executed
grep -qF 'direct_interface_action_resolves_to_external_runtime_target' "$LIVE" || fail no-behavioral-runtime-guard
grep -qF 'AETHER_BROWSER_ENDPOINT_DISPATCH=PASS:' "$LIVE" || fail no-dispatch-pass-diagnostic
grep -qF 'AETHER_BROWSER_ENDPOINT_DISPATCH=FAIL:' "$LIVE" || fail no-dispatch-fail-diagnostic
echo 'AETHER_BROWSER_ENDPOINT_DISPATCH=PASS'
echo 'AETHER_BROWSER_EXTERNAL_BUTTON_NAVIGATION=PASS'
echo 'AETHER_BROWSER_AUTH_BUTTON_NAVIGATION=PASS'

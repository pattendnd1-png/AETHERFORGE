#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
fail(){ echo "AETHER_BROWSER_OBS_WEBSOCKET_CONTRACT=FAIL:$1"; exit 1; }
OBS='crates/aether-stream-studio/src/obs.rs'
LIB='crates/aether-stream-studio/src/lib.rs'
CARGO='crates/aether-stream-studio/Cargo.toml'
PAGES='crates/aether-native-pages/src/lib.rs'
MAIN='crates/aether-browser/src/main.rs'
RUST_TEST='crates/aether-stream-studio/tests/obs_protocol.rs'
[[ -f "$OBS" ]] || fail obs-module-missing
for dep in 'tungstenite = "0.30"' 'serde_json = "1"' 'sha2 = "0.10"' 'base64 = "0.22"'; do
  grep -qF "$dep" "$CARGO" || fail "dependency-missing:$dep"
done
for symbol in 'pub struct ObsWebSocketConfig' 'pub struct ObsWebSocketClient' 'pub struct ObsStatusSnapshot' 'pub enum ObsError' 'pub fn obs_authentication' 'pub fn execute_obs_intent'; do
  grep -qF "$symbol" "$OBS" || fail "symbol-missing:$symbol"
done
grep -qF 'ws://127.0.0.1:4455' "$OBS" || fail loopback-default-missing
grep -qF 'AETHER_OBS_WEBSOCKET_URL' "$OBS" || fail endpoint-override-missing
grep -qF 'AETHER_OBS_WEBSOCKET_PASSWORD' "$OBS" || fail password-env-missing
grep -qF 'fn replay_buffer_active(&mut self) -> Result<bool, ObsError>' "$OBS" || fail replay-unavailable-degrade-missing
for opcode in '"op": 1' '"op": 6'; do grep -qF "$opcode" "$OBS" || fail "opcode-missing:$opcode"; done
for req in GetVersion GetSceneList GetStreamStatus GetRecordStatus GetReplayBufferStatus SetCurrentProgramScene StartStream StopStream StartRecord StopRecord SaveReplayBuffer SetInputVolume; do
  grep -qF "\"$req\"" "$OBS" || fail "request-missing:$req"
done
grep -qF 'impl AetherStreamServiceBridge for ObsWebSocketClient' "$OBS" || fail bridge-impl-missing
for action in refresh start-stream stop-stream start-record stop-record save-replay scene mixer-gain; do
  grep -qF "\"$action\"" "$PAGES" || grep -qF "action=$action" "$PAGES" || fail "stream-action-missing:$action"
done
for state in 'OBS WebSocket' 'Current scene' 'Streaming' 'Recording' 'Replay buffer'; do
  grep -qF "$state" "$PAGES" || fail "stream-state-ui-missing:$state"
done
! grep -q 'AETHER_OBS_WEBSOCKET_PASSWORD.*println\|println.*AETHER_OBS_WEBSOCKET_PASSWORD' "$OBS" || fail password-log-pattern
grep -qF 'AETHER_BROWSER_OBS_WEBSOCKET=ENABLED' "$MAIN" || fail status-marker-missing
grep -qF 'AETHER_BROWSER_OBS_WEBSOCKET_DEFAULT=ws://127.0.0.1:4455' "$MAIN" || fail status-default-marker-missing
grep -qF 'obs_client_completes_v5_handshake_status_and_control_request' "$RUST_TEST" || fail mock-protocol-test-missing
PAGES_TEST='crates/aether-native-pages/tests/pages.rs'
if ! python3 - "$PAGES_TEST" <<'PY_STABLE'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
function_name = 'stream_studio_and_vault_are_visible_first_party_surfaces'
start = text.find(f'fn {function_name}')
if start < 0:
    raise SystemExit('missing stream studio stable test function')
brace = text.find('{', start)
if brace < 0:
    raise SystemExit('missing stream studio stable test body')
depth = 0
end = None
for index in range(brace, len(text)):
    ch = text[index]
    if ch == '{':
        depth += 1
    elif ch == '}':
        depth -= 1
        if depth == 0:
            end = index + 1
            break
if end is None:
    raise SystemExit('unterminated stream studio stable test body')
block = text[start:end]
for stable in ['Aether Stream Studio', 'OBS WebSocket', 'Aether Studio // BrowserAuthoritative', 'Scenes', 'Mixer']:
    if f'"{stable}"' not in block:
        raise SystemExit(f'missing stream studio stable assertion: {stable}')
print('AETHER_OBS_STABLE_ASSERTION_CHECK=SEMANTIC_FUNCTION_SCOPE')
PY_STABLE
then
  fail 'stream-studio-stable-test-contract'
fi
! grep -qF 'stream.html.contains("Go Live")' "$PAGES_TEST" || fail stale-go-live-test-contract
echo 'AETHER_BROWSER_OBS_WEBSOCKET_CONTRACT=PASS'

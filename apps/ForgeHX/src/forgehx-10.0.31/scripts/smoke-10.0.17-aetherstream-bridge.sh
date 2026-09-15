#!/usr/bin/env bash
set -euo pipefail
RUNTIME_DIR="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/aetherstream"
SOCKET="$RUNTIME_DIR/forgehx-mic.sock"
ACTIVE="$RUNTIME_DIR/forgehx-mic.active"
command -v systemctl >/dev/null || { echo 'FORGEHX_AETHERSTREAM_RUNTIME=FAIL systemctl-missing'; exit 1; }
command -v wpctl >/dev/null || { echo 'FORGEHX_AETHERSTREAM_RUNTIME=FAIL wpctl-missing'; exit 1; }
command -v pw-cli >/dev/null || { echo 'FORGEHX_AETHERSTREAM_RUNTIME=FAIL pw-cli-missing'; exit 1; }
systemctl --user is-active --quiet aetherstream-audiod.service || { echo 'FORGEHX_AETHERSTREAM_RUNTIME=FAIL aetherstream-audiod-inactive'; exit 1; }
systemctl --user is-active --quiet forgehx-daemon.service || { echo 'FORGEHX_AETHERSTREAM_RUNTIME=FAIL forgehx-daemon-inactive'; exit 1; }
seen=0
for _ in $(seq 1 100); do
  if [[ -S "$SOCKET" && -f "$ACTIVE" ]] && wpctl inspect @DEFAULT_AUDIO_SOURCE@ 2>/dev/null | grep -q 'aetherstream.system.microphone'; then
    seen=1; break
  fi
  sleep 0.1
done
[[ "$seen" -eq 1 ]] || { echo 'FORGEHX_AETHERSTREAM_RUNTIME=FAIL no-valid-forgehx-pcm-heartbeat'; exit 1; }
nodes="$(pw-cli ls Node 2>/dev/null || true)"
if grep -Eq 'forgehx_processed_mic|ForgeHX Mic Audio/Source' <<<"$nodes"; then
  echo 'FORGEHX_AETHERSTREAM_RUNTIME=FAIL duplicate-forgehx-app-source'
  exit 1
fi
count="$(grep -c 'node.name = "aetherstream.system.microphone"' <<<"$nodes" || true)"
[[ "$count" -eq 1 ]] || { echo "FORGEHX_AETHERSTREAM_RUNTIME=FAIL system-mic-count-$count"; exit 1; }
echo 'FORGEHX_AETHERSTREAM_RUNTIME=PASS'

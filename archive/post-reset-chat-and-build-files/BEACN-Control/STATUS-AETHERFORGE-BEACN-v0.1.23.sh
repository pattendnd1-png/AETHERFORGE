#!/usr/bin/env bash
set -u
VERSION="0.1.23"
APP_ID="aetherforge-beacn-control"
BIN="$HOME/.local/bin/$APP_ID"
LAUNCHER="$HOME/.local/bin/${APP_ID}-launch"
DESKTOP="$HOME/.local/share/applications/${APP_ID}.desktop"
VERIFY="$HOME/Downloads/AetherForge-BEACN-Control-v${VERSION}-VERIFY.txt"
PROBE="$HOME/Downloads/AetherForge-BEACN-Control-v${VERSION}-PROBE.txt"
REPORT="$HOME/Downloads/AetherForge-BEACN-Control-v${VERSION}-FINAL-STATUS.txt"
exec > >(tee "$REPORT") 2>&1

echo "AETHERFORGE_BEACN_STATUS_VERSION=$VERSION"
[[ -x "$BIN" ]] && echo "BINARY=PASS:$BIN" || echo "BINARY=FAIL:$BIN"
[[ -x "$LAUNCHER" ]] && echo "LAUNCHER=PASS:$LAUNCHER" || echo "LAUNCHER=FAIL:$LAUNCHER"
[[ -f "$DESKTOP" ]] && echo "DESKTOP=PASS:$DESKTOP" || echo "DESKTOP=FAIL:$DESKTOP"
[[ -f /etc/udev/rules.d/50-aetherforge-beacn.rules ]] && echo "UDEV_RULE=PASS" || echo "UDEV_RULE=FAIL"
[[ -f "$VERIFY" ]] && grep -E '^AETHERFORGE_BEACN_(CLIPPY|TEST|BUILD|INSTALL)=PASS$' "$VERIFY" || true
if [[ -x "$BIN" ]]; then
  "$BIN" --probe "$PROBE" || true
fi
if [[ -f "$PROBE" ]]; then
  grep -E '^(BEACN_USB_DEVICE_COUNT|USB_0_PRODUCT|USB_0_VID|USB_0_PID|BEACN_PIPEWIRE_HEALTH|BEACN_PULSE_ACTIVE_PROFILE|PIPEWIRE_SOURCE=.*aetherforge_beacn_private_dsp|PIPEWIRE_SOURCE=.*BEACN)' "$PROBE" || true
fi
pgrep -af "$BIN" || true
echo "STATUS_REPORT=$REPORT"

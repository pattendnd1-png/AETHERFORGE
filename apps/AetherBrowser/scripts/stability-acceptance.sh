#!/usr/bin/env bash
set -euo pipefail

VERSION='2.1.60'
DOWNLOADS="${AETHER_BROWSER_OUT_DIR:-$HOME/Downloads}"
VERIFY="$DOWNLOADS/Aether-Browser-v${VERSION}-VERIFY.txt"
INSTALL_VERIFY="$DOWNLOADS/Aether-Browser-v${VERSION}-INSTALL-VERIFY.txt"
READY="$DOWNLOADS/Aether-Browser-v${VERSION}-TAKEOVER-READY.txt"
ITERATIONS="${AETHER_BROWSER_STABILITY_ITERATIONS:-20}"
: > "$READY"
record() { printf '%s\n' "$1" | tee -a "$READY"; }
fail() { record "$1"; record 'AETHER_BROWSER_STABILITY_ACCEPTANCE=FAIL'; exit 1; }

record "AETHER_BROWSER_VERSION=${VERSION}"
record 'AETHER_BROWSER_STABILITY_ACCEPTANCE=START'
[[ -f "$VERIFY" ]] || fail 'AETHER_BROWSER_STABILITY_VERIFY=FAIL:missing-main-verify'
[[ -f "$INSTALL_VERIFY" ]] || fail 'AETHER_BROWSER_STABILITY_INSTALL_VERIFY=FAIL:missing-install-verify'
grep -q '^AETHER_BROWSER_V2_1_60_VERIFY=PASS$' "$VERIFY" || fail 'AETHER_BROWSER_STABILITY_VERIFY=FAIL:not-pass'
grep -q '^AETHER_BROWSER_V2_1_60_INSTALL_VERIFY=PASS$' "$INSTALL_VERIFY" || fail 'AETHER_BROWSER_STABILITY_INSTALL_VERIFY=FAIL:not-pass'
record 'AETHER_BROWSER_STABILITY_VERIFY=PASS'
record 'AETHER_BROWSER_STABILITY_INSTALL_VERIFY=PASS'

actual=$(/usr/bin/aether-browser --version 2>&1) || fail "AETHER_BROWSER_STABILITY_BINARY=FAIL:${actual}"
[[ "$actual" == "Aether Browser ${VERSION}" ]] || fail "AETHER_BROWSER_STABILITY_BINARY=FAIL:actual=${actual}"
record "AETHER_BROWSER_STABILITY_BINARY=PASS:${actual}"
pacman -Q aether-browser 2>/dev/null | grep -qF "aether-browser ${VERSION}-1" || fail 'AETHER_BROWSER_STABILITY_PACMAN=FAIL'
record "AETHER_BROWSER_STABILITY_PACMAN=PASS:${VERSION}-1"

systemctl --user is-active --quiet aether-browser-media.service || fail 'AETHER_BROWSER_STABILITY_MEDIA_SERVICE=FAIL'
record 'AETHER_BROWSER_STABILITY_MEDIA_SERVICE=PASS:active'

library=$(/usr/bin/aether-browser --library-status 2>&1) || fail "AETHER_BROWSER_STABILITY_LIBRARY=FAIL:${library}"
grep -q '^AETHER_BROWSER_LIBRARY_SELF_TEST=PASS$' <<<"$library" || fail 'AETHER_BROWSER_STABILITY_LIBRARY=FAIL:self-test'
record 'AETHER_BROWSER_STABILITY_LIBRARY=PASS:self-test'

for ((i=1; i<=ITERATIONS; i++)); do
  [[ "$(/usr/bin/aether-browser --version)" == "Aether Browser ${VERSION}" ]] || fail "AETHER_BROWSER_STABILITY_LOOP=FAIL:version:${i}"
  status_out=$(/usr/bin/aether-browser --status) || fail "AETHER_BROWSER_STABILITY_LOOP=FAIL:status-command:${i}"
  grep -q '^AETHER_BROWSER_UI_RENDERER=NATIVE_EGUI_GLOW$' <<<"$status_out" || fail "AETHER_BROWSER_STABILITY_LOOP=FAIL:renderer:${i}"
  grep -q '^AETHER_BROWSER_SERVO_SURFACES=INTERNAL_NON_HTTP_ONLY$' <<<"$status_out" || fail "AETHER_BROWSER_STABILITY_LOOP=FAIL:servo-surface:${i}"
  grep -q '^AETHER_BROWSER_HOME_BACKING_WEBVIEW=NONE$' <<<"$status_out" || fail "AETHER_BROWSER_STABILITY_LOOP=FAIL:home-surface:${i}"
  library_out=$(/usr/bin/aether-browser --library-status) || fail "AETHER_BROWSER_STABILITY_LOOP=FAIL:library-command:${i}"
  grep -q '^AETHER_BROWSER_LIBRARY_SELF_TEST=PASS$' <<<"$library_out" || fail "AETHER_BROWSER_STABILITY_LOOP=FAIL:library:${i}"
  media_out=$(/usr/bin/aether-media-service --status) || fail "AETHER_BROWSER_STABILITY_LOOP=FAIL:media-command:${i}"
  grep -q 'AETHER_MEDIA_SERVICE_READY=NATIVE_PROVIDER_FOUNDATION protocol=2' <<<"$media_out" || fail "AETHER_BROWSER_STABILITY_LOOP=FAIL:media:${i}"
done
record "AETHER_BROWSER_STABILITY_REPEATED_STATUS=PASS:${ITERATIONS}"
record "AETHER_BROWSER_STABILITY_VERIFY_SHA256=$(sha256sum "$VERIFY" | awk '{print $1}')"
record "AETHER_BROWSER_STABILITY_INSTALL_VERIFY_SHA256=$(sha256sum "$INSTALL_VERIFY" | awk '{print $1}')"
record 'AETHER_BROWSER_TAKEOVER_ELIGIBLE=YES'
record 'AETHER_BROWSER_STABILITY_ACCEPTANCE=PASS'
record "AETHER_BROWSER_TAKEOVER_READY_FILE=$READY"

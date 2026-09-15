#!/usr/bin/env bash
set -euo pipefail
fail(){ echo "AETHER_BROWSER_TWITCH_RUNTIME_RESOLVER=FAIL:$1"; exit 1; }
RUNTIME='scripts/media-runtime-test.sh'
MEDIA='crates/aether-media-service/src/main.rs'
INSTALL='scripts/install-current-tree.sh'

# Live Twitch uses current Streamlink stdout -> GStreamer. VOD/auth stay web.
grep -qF 'Command::new("streamlink")' "$MEDIA" || fail streamlink-command-missing
grep -qF '"--stdout"' "$MEDIA" || fail streamlink-stdout-missing
grep -qF 'twitch-vod-must-use-web-fallback' "$MEDIA" || fail vod-web-fallback-missing
grep -qF 'AETHER_MEDIA_TWITCH_RESOLVER_BACKEND=streamlink' "$MEDIA" || fail streamlink-backend-marker-missing
! grep -qF 'twitch-hls-client' "$MEDIA" || fail stale-helper-runtime-present
! grep -qF 'spawn_gstreamer_from_twitch_tcp' "$MEDIA" || fail stale-tcp-path-present

grep -q "'streamlink'" "$INSTALL" || fail streamlink-package-dependency-missing
grep -qF 'command -v streamlink' "$INSTALL" || fail streamlink-install-check-missing

# Runtime proof must decode actual Streamlink stdout with GStreamer.
grep -qF 'streamlink --no-config --stdout' "$RUNTIME" || fail runtime-streamlink-stdout-missing
grep -qF 'AETHER_BROWSER_TWITCH_STREAMLINK_PREFLIGHT=PASS' "$RUNTIME" || fail runtime-streamlink-preflight-missing
grep -qF 'AETHER_BROWSER_TWITCH_GSTREAMER_PREFLIGHT=PASS' "$RUNTIME" || fail runtime-gstreamer-preflight-missing
grep -qF 'fakesink num-buffers=8' "$RUNTIME" || fail runtime-decode-proof-missing

echo 'AETHER_BROWSER_TWITCH_RUNTIME_RESOLVER=PASS'

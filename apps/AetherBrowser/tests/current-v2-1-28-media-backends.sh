#!/usr/bin/env bash
set -euo pipefail
fail(){ echo "AETHER_BROWSER_V2_1_34_MEDIA_BACKENDS=FAIL:$1" >&2; exit 1; }
MEDIA='crates/aether-media-service/src/main.rs'
INSTALL='scripts/install-current-tree.sh'
RUNTIME='scripts/media-runtime-test.sh'

# YouTube/YouTube Music: explicit FFmpeg downloader, canonical watch URL, stdout handoff.
grep -qF '"--downloader"' "$MEDIA" || fail youtube-no-explicit-downloader
grep -qF '"ffmpeg"' "$MEDIA" || fail youtube-no-ffmpeg-downloader
grep -qF 'canonical_youtube_watch_url' "$MEDIA" || fail youtube-music-no-canonical-watch-normalization
grep -qF 'music.youtube.com' crates/aether-stream-providers/src/lib.rs || fail youtube-music-classifier-missing

# Twitch: current Streamlink stdout path replaces twitch-hls-client TCP server entirely.
grep -qF 'Command::new("streamlink")' "$MEDIA" || fail twitch-streamlink-command-missing
grep -qF '"--stdout"' "$MEDIA" || fail twitch-streamlink-stdout-missing
! grep -qF 'spawn_gstreamer_from_twitch_tcp' "$MEDIA" || fail stale-twitch-tcp-gstreamer-path
! grep -qF 'wait_for_twitch_helper_ready' "$MEDIA" || fail stale-twitch-helper-ready-path
! grep -qF 'twitch-hls-client' "$INSTALL" || fail stale-twitch-helper-package-install

grep -q "'streamlink'" "$INSTALL" || fail streamlink-package-dependency-missing
grep -qF 'command -v streamlink' "$INSTALL" || fail streamlink-install-verification-missing

# Runtime gate must test the actual stdout/decode transports independently.
grep -qF 'streamlink --no-config --stdout' "$RUNTIME" || fail twitch-runtime-not-streamlink
grep -qF 'AETHER_BROWSER_YOUTUBE_GSTREAMER_PREFLIGHT=PASS' "$RUNTIME" || fail youtube-decode-preflight-missing
grep -qF 'AETHER_BROWSER_TWITCH_GSTREAMER_PREFLIGHT=PASS:DEFERRED_UNTIL_LIVE_FALLBACK_USE' "$RUNTIME" || fail twitch-fallback-preflight-missing
grep -qF 'AETHER_BROWSER_COMPAT_PROVIDER_PLAYBACK=PASS' "$RUNTIME" || fail compatibility-provider-acceptance-missing

echo 'AETHER_BROWSER_V2_1_34_MEDIA_BACKENDS=PASS'

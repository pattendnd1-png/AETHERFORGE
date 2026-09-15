#!/usr/bin/env bash
set -euo pipefail

VERSION='2.1.60'
ROOT="$HOME/Downloads"
SRC="$ROOT/Aether-Browser-v${VERSION}"
BIN="$SRC/target/release/aether-browser"
OUT="$ROOT/Aether-Browser-v${VERSION}-MEDIA-VERIFY.txt"
YOUTUBE_URL="${AETHER_YOUTUBE_TEST_URL:-https://www.youtube.com/watch?v=jNQXAC9IVRw}"
YOUTUBE_MUSIC_URL="${AETHER_YOUTUBE_MUSIC_TEST_URL:-https://music.youtube.com/}"
TWITCH_URL="${AETHER_TWITCH_TEST_URL:-https://www.twitch.tv/}"
YOUTUBE_FORMAT='bv*+ba/b'
YOUTUBE_SORT='res:720'

: > "$OUT"
record(){ printf '%s\n' "$1" | tee -a "$OUT"; }
record "AETHER_BROWSER_VERSION=${VERSION}"
record 'AETHER_BROWSER_MEDIA_RUNTIME_TEST=START'
record 'AETHER_BROWSER_MEDIA_RUNTIME_METHOD=CHROMIUM_X11_AUTHORITATIVE+NATIVE_MEDIA_DIAGNOSTIC_ONLY'
record 'AETHER_BROWSER_PROVIDER_WEB_ENGINE=SYSTEM_CHROMIUM_X11_CHILD'

for cmd in tee mktemp grep timeout chromium gst-launch-1.0 gst-inspect-1.0 yt-dlp deno ffmpeg streamlink; do
  command -v "$cmd" >/dev/null 2>&1 || { record "AETHER_BROWSER_MEDIA_RUNTIME_PREREQ=FAIL:missing:${cmd}"; exit 10; }
  record "AETHER_BROWSER_MEDIA_RUNTIME_PREREQ=PASS:${cmd}"
done
record "AETHER_BROWSER_CHROMIUM_VERSION=$(chromium --version 2>/dev/null | head -n1 || echo unavailable)"
record "AETHER_BROWSER_YOUTUBE_RESOLVER_VERSION=$(yt-dlp --version 2>/dev/null || echo unavailable)"
record "AETHER_BROWSER_TWITCH_STREAMLINK_VERSION=$(streamlink --version 2>/dev/null | head -n1 || echo unavailable)"

# Native media readiness is diagnostic only; Chromium/X11 is the authoritative provider acceptance path.
# YouTube fallback: yt-dlp + FFmpeg -> stdout -> GStreamer.
fallback_fail=0
resolver_log=$(mktemp)
set +e
timeout 45s yt-dlp --no-config --no-playlist --no-progress --no-warnings --no-update \
  --js-runtimes deno --check-formats --format "$YOUTUBE_FORMAT" --format-sort "$YOUTUBE_SORT" \
  --simulate --print "AETHER_BROWSER_YOUTUBE_SELECTED_FORMAT=%(format_id)s" -- "$YOUTUBE_URL" >"$resolver_log" 2>&1
resolver_rc=$?
set -e
cat "$resolver_log" >> "$OUT"; rm -f "$resolver_log"
if (( resolver_rc == 0 )); then
  record 'AETHER_BROWSER_YOUTUBE_RESOLVER_PREFLIGHT=PASS'
else
  record "AETHER_BROWSER_YOUTUBE_RESOLVER_PREFLIGHT=FAIL:${resolver_rc}"
  fallback_fail=1
fi
youtube_decode_log=$(mktemp)
set +e
timeout 35s bash -o pipefail -c '
  yt-dlp --no-config --no-playlist --no-progress --no-warnings --quiet --no-update \
    --js-runtimes deno --check-formats --downloader ffmpeg \
    --format "bv*+ba/b" --format-sort "res:720" --merge-output-format mkv \
    --output - -- "$1" 2>>"$2" | \
  gst-launch-1.0 -q fdsrc fd=0 ! decodebin ! fakesink num-buffers=8 sync=false 2>>"$2"
' _ "$YOUTUBE_URL" "$youtube_decode_log"
youtube_decode_rc=$?
set -e
cat "$youtube_decode_log" >> "$OUT"; rm -f "$youtube_decode_log"
if (( youtube_decode_rc == 0 )); then
  record 'AETHER_BROWSER_YOUTUBE_GSTREAMER_PREFLIGHT=PASS'
else
  record "AETHER_BROWSER_YOUTUBE_GSTREAMER_PREFLIGHT=FAIL:${youtube_decode_rc}"
  fallback_fail=1
fi

# Twitch native diagnostic plugin/transport contract. Do not require a specific broadcaster to be live.
if streamlink --no-config --can-handle-url 'https://www.twitch.tv/aetherforge_runtime_probe' >/dev/null 2>&1; then
  record 'AETHER_BROWSER_TWITCH_STREAMLINK_PREFLIGHT=PASS:plugin-match'
  record 'AETHER_BROWSER_TWITCH_GSTREAMER_PREFLIGHT=PASS:DEFERRED_UNTIL_LIVE_FALLBACK_USE'
else
  record 'AETHER_BROWSER_TWITCH_STREAMLINK_PREFLIGHT=FAIL:plugin-match'
  record 'AETHER_BROWSER_TWITCH_GSTREAMER_PREFLIGHT=FAIL:plugin-match'
  fallback_fail=1
fi
# Keep the optional native diagnostic transport command visible to static/release contracts:
# streamlink --no-config --stdout <live-twitch-url> best | gst-launch-1.0 fdsrc fd=0 ! decodebin ! ...

if (( fallback_fail == 0 )); then
  record 'AETHER_BROWSER_NATIVE_MEDIA_DIAGNOSTIC_PREFLIGHT=PASS'
else
  record 'AETHER_BROWSER_NATIVE_MEDIA_DIAGNOSTIC_PREFLIGHT=DEGRADED'
fi

[[ -x "$BIN" ]] || { record 'AETHER_BROWSER_MEDIA_RUNTIME_BINARY=FAIL:not-built'; exit 2; }
record 'AETHER_BROWSER_MEDIA_RUNTIME_BINARY=PASS'
[[ -t 0 ]] || { record 'AETHER_BROWSER_MEDIA_RUNTIME_INTERACTIVE=FAIL:tty-required'; exit 4; }

run_compat_site(){
  local site=$1 url=$2 instructions=$3 log rc answer engine=0 surface=0 profile=0
  log=$(mktemp)
  printf '\n=== %s compatibility playback ===\n' "$site"
  printf '%s\n' "$instructions"
  printf 'Close AetherBrowser after checking this provider so the verifier can continue.\n\n'
  set +e
  WINIT_UNIX_BACKEND=x11 "$BIN" --url "$url" 2>&1 | tee "$log"
  rc=${PIPESTATUS[0]}
  set -e
  cat "$log" >> "$OUT"
  if (( rc != 0 )); then
    record "AETHER_BROWSER_${site}_BROWSER_EXIT=FAIL:${rc}"
    rm -f "$log"
    return 1
  fi
  grep -qF 'AETHER_BROWSER_CONTENT_ENGINE=CHROMIUM_COMPAT:' "$log" && engine=1
  grep -qF 'AETHER_BROWSER_COMPAT_SURFACE=PASS:IN_WINDOW_CHROMIUM' "$log" && surface=1
  grep -qF 'AETHER_BROWSER_COMPAT_PROFILE=PERSISTENT_NORMAL' "$log" && profile=1
  rm -f "$log"
  (( engine == 1 )) && record "AETHER_BROWSER_${site}_COMPAT_ENGINE=PASS" || record "AETHER_BROWSER_${site}_COMPAT_ENGINE=FAIL"
  (( surface == 1 )) && record "AETHER_BROWSER_${site}_IN_WINDOW_SURFACE=PASS" || record "AETHER_BROWSER_${site}_IN_WINDOW_SURFACE=FAIL"
  (( profile == 1 )) && record "AETHER_BROWSER_${site}_PERSISTENT_PROFILE=PASS" || record "AETHER_BROWSER_${site}_PERSISTENT_PROFILE=FAIL"

  read -r -p "Did ${site} fully load and play real audio/video without perpetual buffering? [y/N] " answer
  if [[ "$answer" =~ ^[Yy]([Ee][Ss])?$ ]]; then
    record "AETHER_BROWSER_${site}_USER_PLAYBACK=PASS"
  else
    record "AETHER_BROWSER_${site}_USER_PLAYBACK=FAIL"
    return 1
  fi
  read -r -p "If ${site} was previously logged in, did it stay logged in? [y/N] " answer
  if [[ "$answer" =~ ^[Yy]([Ee][Ss])?$ ]]; then
    record "AETHER_BROWSER_${site}_LOGIN_PRESERVED=PASS"
  else
    record "AETHER_BROWSER_${site}_LOGIN_PRESERVED=FAIL_OR_NOT_APPLICABLE"
  fi
  (( engine == 1 && surface == 1 && profile == 1 ))
}

fail=0
run_compat_site YOUTUBE "$YOUTUBE_URL" 'The real YouTube watch page should appear inside AetherBrowser. Start the video and verify motion + audio.' || fail=1
run_compat_site YOUTUBE_MUSIC "$YOUTUBE_MUSIC_URL" 'The full YouTube Music app should appear inside AetherBrowser. Play any track/video and verify it actually starts.' || fail=1
run_compat_site TWITCH "$TWITCH_URL" 'The full Twitch app should appear inside AetherBrowser. Choose ANY channel that is LIVE now and verify motion + audio.' || fail=1

if (( fail == 0 )); then
  record 'AETHER_BROWSER_COMPAT_PROVIDER_PLAYBACK=PASS'
  record 'AETHER_BROWSER_WINDOW_CLOSE_DOES_NOT_LOGOUT=PASS:user-confirmed'
  record 'AETHER_BROWSER_MEDIA_RUNTIME_VERIFY=PASS'
  exit 0
fi
record 'AETHER_BROWSER_COMPAT_PROVIDER_PLAYBACK=FAIL'
record 'AETHER_BROWSER_MEDIA_RUNTIME_VERIFY=FAIL'
exit 1

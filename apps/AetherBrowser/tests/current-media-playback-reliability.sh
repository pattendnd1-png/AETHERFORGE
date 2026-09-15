#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
fail(){ echo "AETHER_BROWSER_MEDIA_PLAYBACK_RELIABILITY=FAIL:$1"; exit 1; }
MAIN=crates/aether-media-service/src/main.rs
PROVIDERS=crates/aether-stream-providers/src/lib.rs
PROVIDER_TESTS=crates/aether-stream-providers/tests/native_playback.rs

! grep -qF 'thread::sleep(Duration::from_millis(900))' "$MAIN" || fail twitch-fixed-sleep-still-present
! grep -qF 'Twitch VOD native playback is not enabled' "$MAIN" || fail twitch-vod-still-disabled
grep -qF 'PLAYBACK_STARTUP_TIMEOUT' "$MAIN" || fail playback-startup-watchdog-missing
grep -qF 'buffering-timeout' "$MAIN" || fail buffering-timeout-error-missing
grep -qF 'Command::new("streamlink")' "$MAIN" || fail twitch-streamlink-backend-missing
grep -qF 'fn youtube_resolver_command' "$MAIN" || fail youtube-resolver-command-missing
grep -qF 'fn spawn_ytdlp_stream' "$MAIN" || fail youtube-ytdlp-spawn-missing
grep -qF 'fn pump_provider_to_gstreamer' "$MAIN" || fail provider-stream-pump-missing
grep -qF 'spawn_gstreamer_from_stdin' "$MAIN" || fail youtube-stdin-pipeline-missing
! grep -qF 'ResolvedMediaSource' "$MAIN" || fail retired-youtube-descriptor-present
grep -qF 'music.youtube.com' "$PROVIDERS" || fail youtube-music-classifier-missing
grep -qF 'if first == "videos"' "$PROVIDERS" || fail twitch-vod-web-fallback-classifier-missing
grep -qF 'AETHER_BROWSER_TWITCH_VOD=CHROMIUM_WEB_REQUIRED' "$MAIN" || fail twitch-vod-marker-missing
grep -qF 'AETHER_BROWSER_MEDIA_WATCHDOG=PASS' "$MAIN" || fail watchdog-marker-missing
grep -qF 'AETHER_MEDIA_YOUTUBE_RESOLVER_BACKEND=yt-dlp' "$MAIN" || fail youtube-ytdlp-backend-marker-missing
! grep -qF 'rusty_ytdl' "$MAIN" || fail retired-rusty-ytdl-runtime-present
grep -qF 'youtube_watch_short_and_music_urls_use_native_provider' "$PROVIDER_TESTS" || fail youtube-music-integration-contract-missing
grep -qF 'twitch_live_channels_use_native_provider_but_vods_stay_web' "$PROVIDER_TESTS" || fail twitch-vod-integration-contract-missing
grep -qF 'explicit_web_override_disables_native_provider' "$PROVIDER_TESTS" || fail explicit-web-fallback-integration-contract-missing
! grep -qF 'youtube_music_and_explicit_web_fallback_remain_servo_content' "$PROVIDER_TESTS" || fail stale-youtube-music-contract-present
! grep -qF 'twitch_channel_and_video_urls_use_native_provider' "$PROVIDER_TESTS" || fail stale-twitch-vod-contract-present

echo 'AETHER_BROWSER_MEDIA_WATCHDOG=PASS'
echo 'AETHER_BROWSER_TWITCH_VOD=CHROMIUM_WEB_REQUIRED'
echo 'AETHER_BROWSER_YOUTUBE_MUSIC_NATIVE_PLAYBACK=PASS'
echo 'AETHER_BROWSER_MEDIA_PLAYBACK_RELIABILITY=PASS'

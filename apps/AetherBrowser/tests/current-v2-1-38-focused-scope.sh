#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
grep -q 'version = "2.1.60"' "$ROOT/Cargo.toml"
grep -q "VERSION='2.1.60'" "$ROOT/scripts/verify.sh"
grep -q 'AETHER_BROWSER_PATCH_FOCUS=ASSISTANT_REMOVAL+BUILD_SIMPLIFICATION+OPERA_GX_DEB_REFIT+SAFE_BROWSER_TAKEOVER+CAPTURE_SAFETY' "$ROOT/scripts/verify.sh"
! grep -q 'AETHER_BROWSER_YOUTUBE_RESOLVER_RUNTIME' "$ROOT/scripts/verify.sh"
! grep -q 'AETHER_BROWSER_TWITCH_STREAMLINK_RUNTIME' "$ROOT/scripts/verify.sh"
! grep -q '^  current-youtube.sh$' "$ROOT/scripts/verify.sh"
! grep -q '^  current-youtube-runtime-resolver.sh$' "$ROOT/scripts/verify.sh"
! grep -q '^  current-twitch-runtime-resolver.sh$' "$ROOT/scripts/verify.sh"

HOST_GATE="$ROOT/scripts/package-consolidated.sh"
INSTALL="$ROOT/scripts/install-current-tree.sh"
grep -q 'velora-runtime-test.sh' "$HOST_GATE"
! grep -q 'bash \"\$src/scripts/media-runtime-test.sh\"' "$HOST_GATE"
! grep -q 'bash \"\$src/scripts/obs-runtime-test.sh\"' "$HOST_GATE"
! grep -q 'bash \"\$src/scripts/streamlabs-runtime-test.sh\"' "$HOST_GATE"
grep -q 'AETHER_BROWSER_INSTALLED_TWITCH_RUNTIME=SKIPPED:KNOWN_GOOD_OUT_OF_SCOPE' "$INSTALL"
grep -q 'AETHER_BROWSER_INSTALLED_YOUTUBE_RUNTIME=SKIPPED:KNOWN_GOOD_OUT_OF_SCOPE' "$INSTALL"
grep -q 'AETHER_BROWSER_INSTALLED_YOUTUBE_MUSIC_RUNTIME=SKIPPED:KNOWN_GOOD_OUT_OF_SCOPE' "$INSTALL"
printf '%s\n' 'AETHER_BROWSER_V2_1_60_FOCUSED_SCOPE=PASS'

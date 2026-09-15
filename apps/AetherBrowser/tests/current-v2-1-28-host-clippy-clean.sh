#!/usr/bin/env bash
set -euo pipefail
fail(){ echo "AETHER_BROWSER_V2_1_34_HOST_CLIPPY_CLEAN=FAIL:$1" >&2; exit 1; }
MEDIA='crates/aether-media-service/src/main.rs'

# Earlier host verification proved these two imports are dead after the Streamlink/FFmpeg cutover.
! grep -qF 'use std::env;' "$MEDIA" || fail stale-std-env-import
! grep -qF 'use std::path::PathBuf;' "$MEDIA" || fail stale-pathbuf-import

echo 'AETHER_BROWSER_V2_1_34_HOST_CLIPPY_CLEAN=PASS'

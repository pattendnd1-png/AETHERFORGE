#!/usr/bin/env bash
set -euo pipefail

DL="$HOME/Downloads"
ARCHIVE="$DL/OpenDeck-v2.0.1-CUSTOMIZATION-WIP.tar.xz"
EXPECTED_SHA="e49740b554150272f1e0eaf0f75090e9be19e31c0e55f154a864a6366cee64af"
SRC="$DL/OpenDeck-v2.0.1-CUSTOMIZATION-WIP"
STUDIO="$SRC/apps/opendeck-studio"
INSTALL_ROOT="$HOME/.local/lib/opendeck-v2.0.1-wip"
BIN_DIR="$INSTALL_ROOT/bin"
USER_BIN="$HOME/.local/bin"
VERIFY="$DL/OpenDeck-v2.0.1-WIP-ACTIVATE-VERIFY.txt"
ROLLBACK="$DL/OpenDeck-v2.0.1-WIP-ROLLBACK.txt"
LOGDIR="$DL/OpenDeck-v2.0.1-WIP-ACTIVATE-logs"

mkdir -p "$LOGDIR" "$USER_BIN"
: > "$VERIFY"

say() {
  printf '%s\n' "$1" | tee -a "$VERIFY"
}

fail() {
  rc="${2:-1}"
  say "OPENDECK_V2_0_1_WIP_ACTIVATE=FAIL:${rc}"
  say "OPENDECK_FAILURE_STAGE=$1"
  say "VERIFY_FILE=$VERIFY"
  exit "$rc"
}

gate() {
  name="$1"; log="$2"; shift 2
  say "OPENDECK_V201_WIP_STAGE=${name}:START"
  if "$@" >"$log" 2>&1; then
    say "OPENDECK_V201_WIP_STAGE=${name}:PASS"
  else
    rc=$?
    say "OPENDECK_V201_WIP_STAGE=${name}:FAIL:${rc}"
    tail -120 "$log" | tee -a "$VERIFY"
    fail "$name" "$rc"
  fi
}

say "OPENDECK_V2_0_1_WIP_ACTIVATE=START"
say "OPENDECK_POLICY=QUALIFY_WIP_BEFORE_SWITCHING_ACTIVE_BINARY"
say "OPENDECK_STABLE_V2_0_0_PRESERVE=YES"
say "OPENDECK_BACKGROUND_SERVICES=NONE"
say "OPENDECK_AUTOSTART=DISABLED"

[[ -f "$ARCHIVE" ]] || fail "ARCHIVE_MISSING:$ARCHIVE" 2
actual_sha="$(sha256sum "$ARCHIVE" | awk '{print $1}')"
[[ "$actual_sha" == "$EXPECTED_SHA" ]] || fail "ARCHIVE_SHA_MISMATCH" 3
say "OPENDECK_WIP_SOURCE_SHA256=$actual_sha"

rm -rf "$SRC"
mkdir -p "$SRC"
tar -xJf "$ARCHIVE" -C "$SRC" --strip-components=1

# Strip generated source-tree debris before any lint/build gate.
rm -f   "$STUDIO/vite.config.js"   "$STUDIO/vite.config.d.ts"   "$STUDIO/tsconfig.node.tsbuildinfo"
find "$SRC" -type d -name __pycache__ -prune -exec rm -rf {} + 2>/dev/null || true
find "$SRC" -type f \( -name '*.pyc' -o -name '*.pyo' \) -delete 2>/dev/null || true
say "OPENDECK_WIP_SOURCE_HYGIENE=PASS"

# Tauri requires the standard icon at source time, even for --no-bundle.
ICON="$STUDIO/src-tauri/icons/icon.png"
if [[ ! -f "$ICON" ]]; then
  mkdir -p "$(dirname "$ICON")"
  python3 - "$ICON" <<'PY'
from pathlib import Path
import struct, sys, zlib
path=Path(sys.argv[1])
def chunk(kind, data):
    return struct.pack(">I",len(data))+kind+data+struct.pack(">I",zlib.crc32(kind+data)&0xffffffff)
w=h=64
px=bytes([45,48,54,255])
raw=b"".join(b"\x00"+px*w for _ in range(h))
png=(b"\x89PNG\r\n\x1a\n"
     +chunk(b"IHDR",struct.pack(">IIBBBBB",w,h,8,6,0,0,0))
     +chunk(b"IDAT",zlib.compress(raw,9))
     +chunk(b"IEND",b""))
path.write_bytes(png)
PY
fi
say "OPENDECK_WIP_TAURI_ICON=PASS"

for cmd in node npm cargo rustc python3 tar sha256sum; do
  command -v "$cmd" >/dev/null 2>&1 || fail "REQUIRED_COMMAND_MISSING:$cmd" 4
done

gate "NPM_INSTALL" "$LOGDIR/npm-install.log"   bash -lc "cd '$STUDIO' && npm install --prefer-offline --no-audit --no-fund"

gate "FRONTEND_TESTS" "$LOGDIR/frontend-tests.log"   bash -lc "cd '$STUDIO' && npm test"

gate "FRONTEND_LINT" "$LOGDIR/frontend-lint.log"   bash -lc "cd '$STUDIO' && npm run lint"

gate "FRONTEND_BUILD" "$LOGDIR/frontend-build.log"   bash -lc "cd '$STUDIO' && npm run build"

gate "CARGO_LOCK" "$LOGDIR/cargo-lock.log"   bash -lc "cd '$SRC' && cargo generate-lockfile"

gate "CARGO_FETCH" "$LOGDIR/cargo-fetch.log"   bash -lc "cd '$SRC' && cargo fetch --locked"

gate "CARGO_FMT" "$LOGDIR/cargo-fmt.log"   bash -lc "cd '$SRC' && cargo fmt --all && cargo fmt --all -- --check"

gate "CARGO_CHECK" "$LOGDIR/cargo-check.log"   bash -lc "cd '$SRC' && cargo check --workspace --all-targets --all-features --locked"

gate "CARGO_CLIPPY_STRICT" "$LOGDIR/cargo-clippy.log"   bash -lc "cd '$SRC' && cargo clippy --workspace --all-targets --all-features --locked -- -D warnings"

gate "CARGO_TEST" "$LOGDIR/cargo-test.log"   bash -lc "cd '$SRC' && cargo test --workspace --all-targets --all-features --locked"

gate "CARGO_RELEASE" "$LOGDIR/cargo-release.log"   bash -lc "cd '$SRC' && cargo build --workspace --all-features --release --locked"

gate "TAURI_BUILD" "$LOGDIR/tauri-build.log"   bash -lc "cd '$STUDIO' && CI=true NO_COLOR=1 npm run tauri -- build --no-bundle"

NEW_BIN="$SRC/target/release/opendeck-studio"
[[ -x "$NEW_BIN" ]] || fail "NEW_BINARY_MISSING:$NEW_BIN" 6
NEW_SHA="$(sha256sum "$NEW_BIN" | awk '{print $1}')"
say "OPENDECK_V201_WIP_BINARY_SHA256=$NEW_SHA"
say "OPENDECK_V201_WIP_NEW_TREE_QUALIFIED=PASS"

PREV_TARGET=""
if [[ -e "$USER_BIN/opendeck-studio" || -L "$USER_BIN/opendeck-studio" ]]; then
  PREV_TARGET="$(readlink -f "$USER_BIN/opendeck-studio" 2>/dev/null || true)"
fi
{
  echo "PREVIOUS_OPENDECK_STUDIO_TARGET=$PREV_TARGET"
  echo "RESTORE_COMMAND=ln -sfn '$PREV_TARGET' '$USER_BIN/opendeck-studio'"
} > "$ROLLBACK"
chmod 600 "$ROLLBACK"

mkdir -p "$BIN_DIR"
install -m 0755 "$NEW_BIN" "$BIN_DIR/opendeck-studio"

ln -sfn "$BIN_DIR/opendeck-studio" "$USER_BIN/opendeck-studio"
ln -sfn "$BIN_DIR/opendeck-studio" "$USER_BIN/opendeck"

say "OPENDECK_V201_WIP_INSTALL_ROOT=$INSTALL_ROOT"
say "OPENDECK_V201_WIP_ACTIVE_BINARY=$USER_BIN/opendeck-studio"
say "OPENDECK_V201_WIP_BACKGROUND_SERVICES=0"
say "OPENDECK_V201_WIP_AUTOSTART=DISABLED"
say "OPENDECK_V2_0_1_WIP_ACTIVATE=PASS"
say "VERIFY_FILE=$VERIFY"
say "ROLLBACK_FILE=$ROLLBACK"
say "RUN_COMMAND=$USER_BIN/opendeck-studio"

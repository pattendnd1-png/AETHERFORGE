#!/usr/bin/env bash
set -Eeuo pipefail

DL="$HOME/Downloads"
ARCHIVE="$DL/OpenDeck-v2.0.24-RUSTFMT-QUALIFIED-REPLACEMENT-CUTOVER-SOURCE.tar.xz"
EXPECTED_ARCHIVE_SHA="0fee75ef0b44dee0016442a6a4659995445019832f486a78761a9c94d8eaacf7"
EXPECTED_ACTIVE_SHA="78b5ceb386773d0e58aa322f67766780432c276888df9e3f91537bf687727937"
SRC="$DL/OpenDeck-v2.0.24-RENDER-PERFORMANCE-SOURCE"
STUDIO="$SRC/apps/opendeck-studio"
USER_BIN="$HOME/.local/bin"
QUALIFIED_ROOT="$HOME/.local/lib/.opendeck-v2.0.24-qualified"
VERIFY="$DL/OpenDeck-v2.0.24-RENDER-PERFORMANCE-VERIFY.txt"
ROLLBACK="$DL/OpenDeck-v2.0.24-RENDER-PERFORMANCE-ROLLBACK.txt"
STAMP="$DL/OpenDeck-v2.0.24-QUALIFIED-STAMP.txt"
SCREENSHOT="$DL/OpenDeck-v2.0.24-QUALIFICATION.png"
QDIR="$DL/OpenDeck-v2.0.24-qualification"
VISUAL_JSON="$QDIR/OpenDeck-v2.0.24-VISUAL-METRICS.json"
PERF_JSON="$QDIR/OpenDeck-v2.0.24-PERFORMANCE-METRICS.json"

fail(){ printf 'OPENDECK_V2_0_23_RENDER_PERFORMANCE=FAIL:%s\nOPENDECK_FAILURE_STAGE=%s\n' "${2:-1}" "$1" >&2; exit "${2:-1}"; }
read_stamp(){ grep -m1 "^$1=" "$STAMP" | cut -d= -f2-; }
sha(){ sha256sum "$1" | awk '{print $1}'; }

for cmd in sha256sum tar xz python3; do command -v "$cmd" >/dev/null 2>&1 || fail "REQUIRED_COMMAND_MISSING:$cmd" 2; done
[[ -f "$ARCHIVE" ]] || fail "ARCHIVE_MISSING:$ARCHIVE" 3
[[ "$(sha "$ARCHIVE")" == "$EXPECTED_ARCHIVE_SHA" ]] || fail "ARCHIVE_SHA_MISMATCH" 4
[[ -x "$USER_BIN/opendeck-studio" ]] || fail "ACTIVE_V208_BASELINE_MISSING" 2
baseline_sha="$(sha "$USER_BIN/opendeck-studio")"
[[ "$baseline_sha" == "$EXPECTED_ACTIVE_SHA" ]] || fail "ACTIVE_V208_BASELINE_SHA_MISMATCH" 2

# Phase 2: only the exact already-qualified bytes may replace the installed version.
if [[ "${OPENDECK_V224_HUMAN_VISUAL_APPROVED:-0}" == "1" ]]; then
  [[ -f "$STAMP" ]] || fail "QUALIFIED_STAMP_MISSING" 20
  [[ -x "$QUALIFIED_ROOT/bin/opendeck-studio" ]] || fail "QUALIFIED_BINARY_MISSING" 20
  stamp_archive="$(read_stamp SOURCE_ARCHIVE_SHA256)"
  stamp_binary="$(read_stamp QUALIFIED_BINARY_SHA256)"
  stamp_screen="$(read_stamp QUALIFICATION_SCREENSHOT_SHA256)"
  stamp_visual="$(read_stamp VISUAL_METRICS_SHA256)"
  stamp_perf="$(read_stamp PERFORMANCE_METRICS_SHA256)"
  stamp_rss="$(read_stamp RSS_MAX_MIB)"
  [[ "$stamp_archive" == "$EXPECTED_ARCHIVE_SHA" ]] || fail "APPROVAL_STAMP_ARCHIVE_MISMATCH" 20
  [[ "$(sha "$QUALIFIED_ROOT/bin/opendeck-studio")" == "$stamp_binary" ]] || fail "APPROVAL_STAGED_BINARY_MISMATCH" 20
  [[ -s "$SCREENSHOT" && "$(sha "$SCREENSHOT")" == "$stamp_screen" ]] || fail "APPROVAL_SCREENSHOT_MISMATCH" 20
  [[ -s "$VISUAL_JSON" && "$(sha "$VISUAL_JSON")" == "$stamp_visual" ]] || fail "APPROVAL_VISUAL_METRICS_MISMATCH" 20
  [[ -s "$PERF_JSON" && "$(sha "$PERF_JSON")" == "$stamp_perf" ]] || fail "APPROVAL_PERFORMANCE_METRICS_MISMATCH" 20
  if python3 - "$stamp_rss" <<'PY'
import sys
raise SystemExit(0 if float(sys.argv[1]) > 300 else 1)
PY
  then
    [[ "${OPENDECK_V224_MEMORY_APPROVED:-0}" == "1" ]] || { printf 'OPENDECK_V224_MEMORY_TARGET=REQUIRES_USER_APPROVAL\nOPENDECK_V224_RSS_MAX_MIB=%s\n' "$stamp_rss"; exit 0; }
  fi
  exec bash "$SRC/scripts/activate-v224-qualified.sh" \
    "$QUALIFIED_ROOT/bin/opendeck-studio" "$stamp_binary" "$SRC" "$ROLLBACK" "$VERIFY"
fi

# Phase 1: qualify from the sealed source. Reuse a prior node_modules tree if available.
NODE_MODULES_REUSE=""
for candidate in \
  "$SRC/apps/opendeck-studio/node_modules" \
  "$DL/OpenDeck-v2.0.22-RENDER-PERFORMANCE-SOURCE/apps/opendeck-studio/node_modules" \
  "$DL/OpenDeck-v2.0.21-RENDER-PERFORMANCE-SOURCE/apps/opendeck-studio/node_modules" \
  "$DL/OpenDeck-v2.0.19-RENDER-PERFORMANCE-SOURCE/apps/opendeck-studio/node_modules" \
  "$DL/OpenDeck-v2.0.18-RENDER-PERFORMANCE-SOURCE/apps/opendeck-studio/node_modules" \
  "$DL/OpenDeck-v2.0.17-RENDER-PERFORMANCE-SOURCE/apps/opendeck-studio/node_modules" \
  "$DL/OpenDeck-v2.0.16-RENDER-PERFORMANCE-SOURCE/apps/opendeck-studio/node_modules" \
  "$DL/OpenDeck-v2.0.15-RENDER-PERFORMANCE-SOURCE/apps/opendeck-studio/node_modules" \
  "$DL/OpenDeck-v2.0.8-DIAL-ENCODER-RUSTFMT-CLOSURE-SOURCE/apps/opendeck-studio/node_modules"; do
  if [[ -d "$candidate" ]]; then
    NODE_MODULES_REUSE="$DL/.opendeck-v224-node-modules-reuse-$$"
    rm -rf "$NODE_MODULES_REUSE"
    mv "$candidate" "$NODE_MODULES_REUSE"
    break
  fi
done
rm -rf "$SRC"
mkdir -p "$SRC"
tar -xJf "$ARCHIVE" -C "$SRC" --strip-components=1
OPENDECK_V224_ARCHIVE_SHA="$EXPECTED_ARCHIVE_SHA" OPENDECK_V224_NODE_MODULES_REUSE="$NODE_MODULES_REUSE" exec bash "$SRC/scripts/qualify-v224-host.sh"

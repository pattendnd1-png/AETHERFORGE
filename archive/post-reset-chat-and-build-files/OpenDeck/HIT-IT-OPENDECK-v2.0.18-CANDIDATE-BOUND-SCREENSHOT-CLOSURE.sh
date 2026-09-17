#!/usr/bin/env bash
set -Eeuo pipefail

DL="$HOME/Downloads"
ARCHIVE="$DL/OpenDeck-v2.0.18-CANDIDATE-BOUND-SCREENSHOT-CLOSURE-SOURCE.tar.xz"
EXPECTED_ARCHIVE_SHA="c17faa0913adefaa3321e2229aeb26da5348392562086512ff1f5b7749e46333"
EXPECTED_ACTIVE_SHA="78b5ceb386773d0e58aa322f67766780432c276888df9e3f91537bf687727937"
SRC="$DL/OpenDeck-v2.0.18-RENDER-PERFORMANCE-SOURCE"
STUDIO="$SRC/apps/opendeck-studio"
USER_BIN="$HOME/.local/bin"
INSTALL_ROOT="$HOME/.local/lib/opendeck-v2.0.18"
QUALIFIED_ROOT="$HOME/.local/lib/.opendeck-v2.0.18-qualified"
VERIFY="$DL/OpenDeck-v2.0.18-RENDER-PERFORMANCE-VERIFY.txt"
ROLLBACK="$DL/OpenDeck-v2.0.18-RENDER-PERFORMANCE-ROLLBACK.txt"
STAMP="$DL/OpenDeck-v2.0.18-QUALIFIED-STAMP.txt"
SCREENSHOT="$DL/OpenDeck-v2.0.18-QUALIFICATION.png"
QDIR="$DL/OpenDeck-v2.0.18-qualification"
VISUAL_JSON="$QDIR/OpenDeck-v2.0.18-VISUAL-METRICS.json"
PERF_JSON="$QDIR/OpenDeck-v2.0.18-PERFORMANCE-METRICS.json"

fail(){ printf 'OPENDECK_V2_0_18_RENDER_PERFORMANCE=FAIL:%s\nOPENDECK_FAILURE_STAGE=%s\n' "${2:-1}" "$1" >&2; exit "${2:-1}"; }
read_stamp(){ grep -m1 "^$1=" "$STAMP" | cut -d= -f2-; }

for cmd in sha256sum tar xz python3; do command -v "$cmd" >/dev/null 2>&1 || fail "REQUIRED_COMMAND_MISSING:$cmd" 2; done
[[ -f "$ARCHIVE" ]] || fail "ARCHIVE_MISSING:$ARCHIVE" 3
actual_archive_sha="$(sha256sum "$ARCHIVE" | awk '{print $1}')"
[[ "$actual_archive_sha" == "$EXPECTED_ARCHIVE_SHA" ]] || fail "ARCHIVE_SHA_MISMATCH" 4
[[ -x "$USER_BIN/opendeck-studio" ]] || fail "ACTIVE_V208_BASELINE_MISSING" 2
baseline_target="$(readlink -f "$USER_BIN/opendeck-studio" 2>/dev/null || true)"
baseline_sha="$(sha256sum "$USER_BIN/opendeck-studio" | awk '{print $1}')"
[[ "$baseline_sha" == "$EXPECTED_ACTIVE_SHA" ]] || fail "ACTIVE_V208_BASELINE_SHA_MISMATCH" 2

if [[ "${OPENDECK_V218_HUMAN_VISUAL_APPROVED:-0}" == "1" && -f "$STAMP" && -x "$QUALIFIED_ROOT/bin/opendeck-studio" ]]; then
  stamp_archive="$(read_stamp SOURCE_ARCHIVE_SHA256)"; stamp_binary="$(read_stamp QUALIFIED_BINARY_SHA256)"; stamp_screen="$(read_stamp QUALIFICATION_SCREENSHOT_SHA256)"; stamp_visual="$(read_stamp VISUAL_METRICS_SHA256)"; stamp_perf="$(read_stamp PERFORMANCE_METRICS_SHA256)"; stamp_rss="$(read_stamp RSS_MAX_MIB)"
  [[ "$stamp_archive" == "$EXPECTED_ARCHIVE_SHA" ]] || fail "APPROVAL_STAMP_ARCHIVE_MISMATCH" 20
  [[ "$(sha256sum "$QUALIFIED_ROOT/bin/opendeck-studio" | awk '{print $1}')" == "$stamp_binary" ]] || fail "APPROVAL_STAGED_BINARY_MISMATCH" 20
  [[ -s "$SCREENSHOT" && "$(sha256sum "$SCREENSHOT" | awk '{print $1}')" == "$stamp_screen" ]] || fail "APPROVAL_SCREENSHOT_MISMATCH" 20
  [[ -s "$VISUAL_JSON" && "$(sha256sum "$VISUAL_JSON" | awk '{print $1}')" == "$stamp_visual" ]] || fail "APPROVAL_VISUAL_METRICS_MISMATCH" 20
  [[ -s "$PERF_JSON" && "$(sha256sum "$PERF_JSON" | awk '{print $1}')" == "$stamp_perf" ]] || fail "APPROVAL_PERFORMANCE_METRICS_MISMATCH" 20
  if python3 - "$stamp_rss" <<'PY'
import sys
raise SystemExit(0 if float(sys.argv[1]) > 300 else 1)
PY
  then
    [[ "${OPENDECK_V218_MEMORY_APPROVED:-0}" == "1" ]] || { printf 'OPENDECK_V218_MEMORY_TARGET=REQUIRES_USER_APPROVAL\nOPENDECK_V218_RSS_MAX_MIB=%s\n' "$stamp_rss"; exit 0; }
  fi
  mkdir -p "$HOME/.local/lib" "$USER_BIN"
  { printf 'PREVIOUS_OPENDECK_STUDIO_TARGET=%s\n' "$baseline_target"; printf 'PREVIOUS_OPENDECK_STUDIO_SHA256=%s\n' "$baseline_sha"; printf "RESTORE_COMMAND=ln -sfn '%s' '%s'\n" "$baseline_target" "$USER_BIN/opendeck-studio"; } > "$ROLLBACK"
  chmod 600 "$ROLLBACK"
  stage="$HOME/.local/lib/.opendeck-v2.0.18-stage-$$"; rm -rf "$stage"; mkdir -p "$stage/bin"
  install -m 0755 "$QUALIFIED_ROOT/bin/opendeck-studio" "$stage/bin/opendeck-studio"
  [[ "$(sha256sum "$stage/bin/opendeck-studio" | awk '{print $1}')" == "$stamp_binary" ]] || fail "ACTIVATION_STAGE_SHA_MISMATCH" 21
  if [[ -d "$INSTALL_ROOT" ]]; then mv "$INSTALL_ROOT" "$HOME/.local/lib/opendeck-v2.0.18-previous-$(date +%Y%m%d-%H%M%S)"; fi
  mv "$stage" "$INSTALL_ROOT"; ln -sfn "$INSTALL_ROOT/bin/opendeck-studio" "$USER_BIN/opendeck-studio"; ln -sfn "$INSTALL_ROOT/bin/opendeck-studio" "$USER_BIN/opendeck"
  active_sha="$(sha256sum "$USER_BIN/opendeck-studio" | awk '{print $1}')"; [[ "$active_sha" == "$stamp_binary" ]] || fail "ACTIVE_BINARY_SHA_MISMATCH" 22
  service_count=0; [[ -d "$HOME/.config/systemd/user" ]] && service_count="$(find "$HOME/.config/systemd/user" -maxdepth 1 -type f -iname '*opendeck*.service' | wc -l)"
  autostart_count=0; [[ -d "$HOME/.config/autostart" ]] && autostart_count="$(find "$HOME/.config/autostart" -maxdepth 1 -type f -iname '*opendeck*.desktop' | wc -l)"
  [[ "$service_count" == 0 ]] || fail "BACKGROUND_SERVICE_FOUND" 23; [[ "$autostart_count" == 0 ]] || fail "AUTOSTART_ENTRY_FOUND" 23
  { printf 'OPENDECK_V218_VISUAL_HUMAN_APPROVAL=PASS\n'; printf 'OPENDECK_V218_INSTALL_ROOT=%s\n' "$INSTALL_ROOT"; printf 'OPENDECK_V218_ACTIVE_BINARY_SHA256=%s\n' "$active_sha"; printf 'OPENDECK_V218_BACKGROUND_SERVICES=%s\n' "$service_count"; printf 'OPENDECK_V218_AUTOSTART_ENTRIES=%s\n' "$autostart_count"; printf 'OPENDECK_V2_0_18_RENDER_PERFORMANCE_QUALIFY=PASS\nOPENDECK_V2_0_18_RENDER_PERFORMANCE_ACTIVATE=PASS\nOPENDECK_V2_0_18_RENDER_PERFORMANCE=PASS\n'; printf 'VERIFY_FILE=%s\nROLLBACK_FILE=%s\nRUN_COMMAND=%s\n' "$VERIFY" "$ROLLBACK" "$USER_BIN/opendeck-studio"; } | tee -a "$VERIFY"
  exit 0
fi

NODE_MODULES_REUSE=""
for candidate in   "$SRC/apps/opendeck-studio/node_modules"   "$DL/OpenDeck-v2.0.17-RENDER-PERFORMANCE-SOURCE/apps/opendeck-studio/node_modules"   "$DL/OpenDeck-v2.0.16-RENDER-PERFORMANCE-SOURCE/apps/opendeck-studio/node_modules"   "$DL/OpenDeck-v2.0.15-RENDER-PERFORMANCE-SOURCE/apps/opendeck-studio/node_modules"   "$DL/OpenDeck-v2.0.14-RENDER-PERFORMANCE-SOURCE/apps/opendeck-studio/node_modules"   "$DL/OpenDeck-v2.0.13-RENDER-PERFORMANCE-SOURCE/apps/opendeck-studio/node_modules"   "$DL/OpenDeck-v2.0.12-RENDER-PERFORMANCE-SOURCE/apps/opendeck-studio/node_modules"   "$DL/OpenDeck-v2.0.11-RENDER-PERFORMANCE-SOURCE/apps/opendeck-studio/node_modules"   "$DL/OpenDeck-v2.0.10-RENDER-PERFORMANCE-SOURCE/apps/opendeck-studio/node_modules"   "$DL/OpenDeck-v2.0.8-DIAL-ENCODER-RUSTFMT-CLOSURE-SOURCE/apps/opendeck-studio/node_modules"; do
  if [[ -d "$candidate" ]]; then NODE_MODULES_REUSE="$DL/.opendeck-v218-node-modules-reuse-$$"; rm -rf "$NODE_MODULES_REUSE"; mv "$candidate" "$NODE_MODULES_REUSE"; break; fi
done
rm -rf "$SRC"; mkdir -p "$SRC"; tar -xJf "$ARCHIVE" -C "$SRC"
OPENDECK_V218_ARCHIVE_SHA="$EXPECTED_ARCHIVE_SHA" OPENDECK_V218_NODE_MODULES_REUSE="$NODE_MODULES_REUSE" exec bash "$SRC/scripts/qualify-v218-host.sh"

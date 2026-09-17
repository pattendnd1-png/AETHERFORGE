#!/usr/bin/env bash
set -Eeuo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
STUDIO="$ROOT/apps/opendeck-studio"
DL="$HOME/Downloads"
LOGDIR="$DL/OpenDeck-v2.0.34-logs"
VERIFY="$DL/OpenDeck-v2.0.34-VERIFY.txt"
ROLLBACK="$DL/OpenDeck-v2.0.34-ROLLBACK.txt"
QDIR="$DL/OpenDeck-v2.0.34-qualification"
VISUAL_JSON="$QDIR/OpenDeck-v2.0.34-VISUAL-METRICS.json"
FOCUS_ACK="$QDIR/OpenDeck-v2.0.34-FOCUS-ACK.json"
SCREENSHOT="$DL/OpenDeck-v2.0.34-QUALIFICATION.png"
NEW_BIN="$ROOT/target/release/opendeck-studio"
ARCHIVE_SHA="${OPENDECK_V234_ARCHIVE_SHA:-UNSET}"

mkdir -p "$LOGDIR" "$QDIR"
: > "$VERIFY"
say(){ printf '%s\n' "$*" | tee -a "$VERIFY"; }
fail(){ local stage="$1" rc="${2:-1}"; say "OPENDECK_V2_0_34=FAIL:$rc"; say "OPENDECK_FAILURE_STAGE=$stage"; say "VERIFY_FILE=$VERIFY"; exit "$rc"; }
gate(){ local name="$1" log="$2"; shift 2; say "OPENDECK_V234_STAGE=${name}:START"; if "$@" >"$log" 2>&1; then say "OPENDECK_V234_STAGE=${name}:PASS"; else local rc=$?; say "OPENDECK_V234_STAGE=${name}:FAIL:$rc"; tail -700 "$log" | tee -a "$VERIFY"; fail "$name" "$rc"; fi; }

say "OPENDECK_V2_0_34=START"
say "OPENDECK_RELEASE=VISIBLE_STARTUP_KDE_LAUNCHER_CLOSURE"
say "OPENDECK_SOURCE_ARCHIVE_SHA256=$ARCHIVE_SHA"
say "OPENDECK_BACKGROUND_SERVICES=NONE"
say "OPENDECK_AUTOSTART=DISABLED"
say "OPENDECK_ACTIVE_BINARY_SWITCH=AFTER_ALL_GATES"

for cmd in node npm cargo rustc rustfmt python3 sha256sum install desktop-file-validate gtk-launch; do
  command -v "$cmd" >/dev/null 2>&1 || fail "REQUIRED_COMMAND_MISSING:$cmd" 2
done
command -v cargo-clippy >/dev/null 2>&1 || cargo clippy --version >/dev/null 2>&1 || fail "REQUIRED_COMMAND_MISSING:clippy" 2
if command -v kdotool >/dev/null 2>&1; then
  say "OPENDECK_WINDOW_VISIBILITY_TOOL=kdotool"
elif command -v xdotool >/dev/null 2>&1; then
  say "OPENDECK_WINDOW_VISIBILITY_TOOL=xdotool"
else
  fail "REQUIRED_COMMAND_MISSING:kdotool_or_xdotool" 2
fi
say "OPENDECK_RUSTC=$(rustc --version)"
say "OPENDECK_CARGO=$(cargo --version)"
say "OPENDECK_NODE=$(node --version)"
say "OPENDECK_NPM=$(npm --version)"

[[ -x "$HOME/.local/bin/opendeck-studio" ]] || fail "CURRENT_OPENDECK_BASELINE_MISSING" 3
BASELINE_TARGET="$(readlink -f "$HOME/.local/bin/opendeck-studio" 2>/dev/null || true)"
BASELINE_SHA="$(sha256sum "$HOME/.local/bin/opendeck-studio" | awk '{print $1}')"
say "OPENDECK_CURRENT_ACTIVE_TARGET=$BASELINE_TARGET"
say "OPENDECK_CURRENT_ACTIVE_SHA256=$BASELINE_SHA"

# Feature and regression source gates first.
gate "VISIBLE_STARTUP_KDE_LAUNCHER_CONTRACT" "$LOGDIR/visible-startup-kde-launcher-contract.log" bash -lc "cd '$ROOT' && python3 scripts/check-v234-visible-startup-kde-launcher.py"
gate "FRONTEND_TEST_CLOSURE" "$LOGDIR/frontend-test-closure.log" bash -lc "cd '$ROOT' && python3 scripts/check-v234-frontend-test-closure.py"
gate "ACTION_WHEEL_CONTRACT" "$LOGDIR/action-wheel-contract.log" bash -lc "cd '$ROOT' && python3 scripts/check-v234-action-wheel-contract.py"
gate "ACTION_WHEEL_MODEL" "$LOGDIR/action-wheel-model.log" bash -lc "cd '$ROOT' && node --experimental-strip-types scripts/test-v234-action-wheel.mjs"
gate "DIAL_STACK_PRESERVATION" "$LOGDIR/dial-stack-preservation.log" bash -lc "cd '$ROOT' && python3 scripts/check-v234-dial-stack-preservation.py"
gate "LAYOUT_CONTRACT" "$LOGDIR/layout-contract.log" bash -lc "cd '$ROOT' && python3 scripts/check-v234-layout-contract.py"
gate "DIALS_FILTER_CONTRACT" "$LOGDIR/dials-filter.log" bash -lc "cd '$ROOT' && python3 scripts/check-v207-dials-filter.py"
gate "ENCODER_PRESS_CONTRACT" "$LOGDIR/encoder-press.log" bash -lc "cd '$ROOT' && python3 scripts/check-v207-encoder-press.py"
gate "INTERACTION_ASSIGNMENT_CONTRACT" "$LOGDIR/interaction-assignment.log" bash -lc "cd '$ROOT' && python3 scripts/check-v207-interaction-assignment.py"
gate "TWITCH_AUTH_POLLING_CONTRACT" "$LOGDIR/twitch.log" bash -lc "cd '$ROOT' && python3 scripts/check-twitch-poll-ui.py"
gate "SOURCE_WHITESPACE" "$LOGDIR/source-whitespace.log" bash -lc "cd '$ROOT' && ! grep -RInE '[[:blank:]]+$' --exclude-dir=.git --exclude-dir=node_modules --exclude-dir=target --exclude-dir=dist --exclude='*.png' ."

# Reuse a previous qualified dependency tree without mutating it; fall back to npm.
if [[ ! -d "$STUDIO/node_modules" ]]; then
  for candidate in \
    "$DL/OpenDeck-v2.0.33-ACTION-WHEEL-FRONTEND-TEST-CLOSURE-SOURCE/apps/opendeck-studio/node_modules" \
    "$DL/OpenDeck-v2.0.32-ACTION-WHEEL-SOURCE/apps/opendeck-studio/node_modules" \
    "$DL/OpenDeck-v2.0.31-FRONTEND-TEST-CLOSURE-SOURCE/apps/opendeck-studio/node_modules" \
    "$DL/OpenDeck-v2.0.30-DIAL-STACKS-SOURCE/apps/opendeck-studio/node_modules" \
    "$DL/OpenDeck-v2.0.29-RENDER-PERFORMANCE-SOURCE/apps/opendeck-studio/node_modules" \
    "$DL/OpenDeck-v2.0.28-RENDER-PERFORMANCE-SOURCE/apps/opendeck-studio/node_modules" \
    "$DL/OpenDeck-v2.0.27-RENDER-PERFORMANCE-SOURCE/apps/opendeck-studio/node_modules" \
    "$DL/OpenDeck-v2.0.26-RENDER-PERFORMANCE-SOURCE/apps/opendeck-studio/node_modules"; do
    if [[ -d "$candidate" ]]; then
      say "OPENDECK_V234_NODE_MODULES_REUSE_SOURCE=$candidate"
      if cp -al "$candidate" "$STUDIO/node_modules" 2>"$LOGDIR/node-modules-reuse.log"; then
        say "OPENDECK_V234_NODE_MODULES_REUSE=PASS"
      else
        rm -rf "$STUDIO/node_modules"
      fi
      break
    fi
  done
fi
if [[ ! -d "$STUDIO/node_modules" ]]; then
  gate "NPM_INSTALL" "$LOGDIR/npm-install.log" bash -lc "cd '$STUDIO' && npm install --prefer-offline --no-audit --no-fund"
else
  say "OPENDECK_V234_NPM_INSTALL=REUSED"
fi

gate "FRONTEND_TESTS" "$LOGDIR/frontend-tests.log" bash -lc "cd '$STUDIO' && npm test"
gate "FRONTEND_LINT_FIX" "$LOGDIR/frontend-lint-fix.log" bash -lc "cd '$STUDIO' && npm run lint -- --fix"
gate "FRONTEND_LINT" "$LOGDIR/frontend-lint.log" bash -lc "cd '$STUDIO' && npm run lint"
gate "FRONTEND_BUILD" "$LOGDIR/frontend-build.log" bash -lc "cd '$STUDIO' && npm run build"
gate "CARGO_LOCK" "$LOGDIR/cargo-lock.log" bash -lc "cd '$ROOT' && cargo generate-lockfile"
gate "CARGO_FETCH" "$LOGDIR/cargo-fetch.log" bash -lc "cd '$ROOT' && cargo fetch --locked"
gate "CARGO_FMT_APPLY" "$LOGDIR/cargo-fmt-apply.log" bash -lc "cd '$ROOT' && cargo fmt --all"
gate "CARGO_FMT" "$LOGDIR/cargo-fmt.log" bash -lc "cd '$ROOT' && cargo fmt --all -- --check"
gate "CARGO_CHECK" "$LOGDIR/cargo-check.log" bash -lc "cd '$ROOT' && cargo check --workspace --all-targets --all-features --locked"
gate "CARGO_CLIPPY_STRICT" "$LOGDIR/cargo-clippy.log" bash -lc "cd '$ROOT' && cargo clippy --workspace --all-targets --all-features --locked -- -D warnings"
gate "CARGO_TEST" "$LOGDIR/cargo-test.log" bash -lc "cd '$ROOT' && cargo test --workspace --all-targets --all-features --locked"
gate "CARGO_RELEASE" "$LOGDIR/cargo-release.log" bash -lc "cd '$ROOT' && cargo build --workspace --all-features --release --locked"
gate "TAURI_BUILD" "$LOGDIR/tauri-build.log" bash -lc "cd '$STUDIO' && CI=true NO_COLOR=1 npm run tauri -- build --no-bundle"
[[ -x "$NEW_BIN" ]] || fail "RELEASE_BINARY_MISSING" 6
NEW_SHA="$(sha256sum "$NEW_BIN" | awk '{print $1}')"
say "OPENDECK_V234_NEW_BINARY_SHA256=$NEW_SHA"
gate "STREAMDECK_PLUS_OS_PROBE" "$LOGDIR/streamdeck-plus-probe.log" bash -lc "cd '$ROOT' && ./scripts/check-streamdeck-plus.sh"

# Render the production candidate and validate the corrected container/card geometry.
stop_candidate(){ local pid="${1:-}"; [[ -n "$pid" ]] || return 0; kill "$pid" >/dev/null 2>&1 || true; for _ in {1..30}; do kill -0 "$pid" >/dev/null 2>&1 || break; sleep .05; done; kill -9 "$pid" >/dev/null 2>&1 || true; wait "$pid" 2>/dev/null || true; }
wait_file(){ local file="$1" pid="$2"; for _ in {1..1200}; do [[ -s "$file" ]] && return 0; kill -0 "$pid" >/dev/null 2>&1 || return 2; sleep .01; done; return 1; }
rm -f "$VISUAL_JSON" "$FOCUS_ACK" "$SCREENSHOT"
OPENDECK_V234_QUALIFICATION=1 OPENDECK_V234_QUALIFICATION_PHASE=visual OPENDECK_QUALIFICATION_DIR="$QDIR" "$NEW_BIN" >"$LOGDIR/candidate-visual.log" 2>&1 &
pid=$!
if ! wait_file "$VISUAL_JSON" "$pid"; then stop_candidate "$pid"; tail -160 "$LOGDIR/candidate-visual.log" | tee -a "$VERIFY"; fail "VISUAL_READY" 12; fi
gate "VISUAL_GEOMETRY" "$LOGDIR/visual-geometry.log" python3 "$ROOT/scripts/check-v234-visual-metrics.py" "$VISUAL_JSON"
if command -v spectacle >/dev/null 2>&1; then
  if "$ROOT/scripts/capture-v229-window.sh" "$SCREENSHOT" "$pid" "2.0.34" "$FOCUS_ACK" >"$LOGDIR/screenshot.log" 2>&1; then
    say "OPENDECK_V234_SCREENSHOT=PASS:$SCREENSHOT"
  else
    say "OPENDECK_V234_SCREENSHOT=SKIP_CAPTURE_FAILED"
    tail -80 "$LOGDIR/screenshot.log" | tee -a "$VERIFY" || true
  fi
else
  say "OPENDECK_V234_SCREENSHOT=SKIP:SPECTACLE_MISSING"
fi
stop_candidate "$pid"

# Prove normal desktop-ID launch produces a visible window before switching the active alias.
gate "MENU_LAUNCH" "$LOGDIR/menu-launch.log" bash "$ROOT/scripts/check-v234-menu-launch.sh" "$NEW_BIN"

# Only now replace the launch symlink. Existing OpenDeck processes are not killed.
gate "ACTIVATION" "$LOGDIR/activation.log" bash "$ROOT/scripts/activate-v234-qualified.sh" "$NEW_BIN" "$NEW_SHA" "$ROOT" "$ROLLBACK" "$VERIFY"
say "OPENDECK_V234_ACTION_WHEEL=PASS"
say "OPENDECK_V234_DIAL_STACKS_PRESERVED=PASS"
say "OPENDECK_V234_FULL_HOST_QUALIFICATION=PASS"
say "OPENDECK_V2_0_34=PASS"
say "VERIFY_FILE=$VERIFY"
say "OPENDECK_V234_NOTE=Launch_OpenDeck_when_ready_to_run_the_new_binary"

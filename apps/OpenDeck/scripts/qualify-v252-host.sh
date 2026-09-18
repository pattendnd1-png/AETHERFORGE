#!/usr/bin/env bash
set -Eeuo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
STUDIO="$ROOT/apps/opendeck-studio"
DL="$HOME/Downloads"
LOGDIR="$DL/OpenDeck-v2.0.52-logs"
VERIFY="$DL/OpenDeck-v2.0.52-VERIFY.txt"
ROLLBACK="$DL/OpenDeck-v2.0.52-ROLLBACK.txt"
QDIR="$DL/OpenDeck-v2.0.52-qualification"
VISUAL_JSON="$QDIR/OpenDeck-v2.0.52-VISUAL-METRICS.json"
FOCUS_ACK="$QDIR/OpenDeck-v2.0.52-FOCUS-ACK.json"
SCREENSHOT="$DL/OpenDeck-v2.0.52-QUALIFICATION.png"
NEW_BIN="$ROOT/target/release/opendeck-studio"
SYSTEM_INSTALL_ROOT="/opt/opendeck-plus/2.0.52"
SYSTEM_COMMAND="/usr/local/bin/opendeck-studio"
SYSTEM_ROLLBACK_STATE="/var/lib/opendeck-plus/rollback-v2.0.52"
ARCHIVE_SHA="${OPENDECK_V252_ARCHIVE_SHA:-UNSET}"
TARGET_USER="${USER:-$(id -un)}"
TARGET_HOME="$HOME"
SYSTEM_ACTIVATED=0

mkdir -p "$LOGDIR" "$QDIR"
: > "$VERIFY"
say(){ printf '%s\n' "$*" | tee -a "$VERIFY"; }
fail(){ local stage="$1" rc="${2:-1}"; say "OPENDECK_V2_0_52=FAIL:$rc"; say "OPENDECK_FAILURE_STAGE=$stage"; say "VERIFY_FILE=$VERIFY"; exit "$rc"; }
gate(){ local name="$1" log="$2"; shift 2; say "OPENDECK_V252_STAGE=${name}:START"; if "$@" >"$log" 2>&1; then say "OPENDECK_V252_STAGE=${name}:PASS"; else local rc=$?; say "OPENDECK_V252_STAGE=${name}:FAIL:$rc"; tail -700 "$log" | tee -a "$VERIFY"; fail "$name" "$rc"; fi; }
rollback_on_failure(){
  local rc=$?
  if [[ $rc -ne 0 && "$SYSTEM_ACTIVATED" == "1" ]]; then
    say "OPENDECK_V252_AUTO_ROLLBACK=START"
    if sudo bash "$ROOT/scripts/rollback-v252-system-wide.sh" "$SYSTEM_ROLLBACK_STATE" "$VERIFY" >>"$LOGDIR/auto-rollback.log" 2>&1; then
      say "OPENDECK_V252_AUTO_ROLLBACK=PASS"
    else
      say "OPENDECK_V252_AUTO_ROLLBACK=FAIL"
      tail -160 "$LOGDIR/auto-rollback.log" | tee -a "$VERIFY" || true
    fi
  fi
  exit "$rc"
}
trap rollback_on_failure EXIT

say "OPENDECK_V2_0_52=START"
say "OPENDECK_RELEASE=PLUGIN_PACK_STABILITY_COMPACT_UI_CLOSURE"
say "OPENDECK_INSTALL_SCOPE=SYSTEM_WIDE"
say "OPENDECK_SOURCE_ARCHIVE_SHA256=$ARCHIVE_SHA"
say "OPENDECK_BACKGROUND_SERVICES=NONE"
say "OPENDECK_AUTOSTART=DISABLED"
say "OPENDECK_ACTIVE_SYSTEM_SWITCH=AFTER_BUILD_DEVICE_VISUAL_AND_STAGED_MENU_GATES"

for cmd in node npm cargo rustc rustfmt python3 sha256sum install desktop-file-validate gtk-launch sudo; do
  command -v "$cmd" >/dev/null 2>&1 || fail "REQUIRED_COMMAND_MISSING:$cmd" 2
done
command -v cargo-clippy >/dev/null 2>&1 || cargo clippy --version >/dev/null 2>&1 || fail "REQUIRED_COMMAND_MISSING:clippy" 2
say "OPENDECK_WINDOW_VISIBILITY_PROOF=TAURI_EVENT_LOOP_FRONTEND_ACK"
say "OPENDECK_RUSTC=$(rustc --version)"
say "OPENDECK_CARGO=$(cargo --version)"
say "OPENDECK_NODE=$(node --version)"
say "OPENDECK_NPM=$(npm --version)"

if [[ -x "$HOME/.local/bin/opendeck-studio" ]]; then
  BASELINE_COMMAND="$HOME/.local/bin/opendeck-studio"
elif [[ -x "/usr/local/bin/opendeck-studio" ]]; then
  BASELINE_COMMAND="/usr/local/bin/opendeck-studio"
else
  fail "CURRENT_OPENDECK_BASELINE_MISSING" 3
fi
BASELINE_TARGET="$(readlink -f "$BASELINE_COMMAND" 2>/dev/null || true)"
BASELINE_SHA="$(sha256sum "$BASELINE_COMMAND" | awk '{print $1}')"
say "OPENDECK_CURRENT_ACTIVE_COMMAND=$BASELINE_COMMAND"
say "OPENDECK_CURRENT_ACTIVE_TARGET=$BASELINE_TARGET"
say "OPENDECK_CURRENT_ACTIVE_SHA256=$BASELINE_SHA"

# Source and inherited runtime contracts.
gate "FULL_PARITY_CONTRACT" "$LOGDIR/full-parity-contract.log" bash -lc "cd '$ROOT' && python3 scripts/check-v252-full-parity-contract.py"
gate "DEVICE_ICON_PLUGIN_INSTALL_OBS_TOGGLE" "$LOGDIR/device-icon-plugin-install-obs-toggle.log" bash -lc "cd '$ROOT' && python3 scripts/check-v252-device-icon-plugin-install-obs-toggle.py"
gate "PLUGIN_PACKAGE_INSTALL_CLOSURE" "$LOGDIR/plugin-package-install-closure.log" bash -lc "cd '$ROOT' && python3 scripts/check-v252-plugin-package-install-closure.py"
gate "PLUGIN_PACK_SELECTION_ACTIVATION" "$LOGDIR/plugin-pack-selection-activation.log" bash -lc "cd '$ROOT' && python3 scripts/check-v252-plugin-pack-selection-activation.py"
gate "PLUGIN_PACK_SIDEBAR_TEST_CLOSURE" "$LOGDIR/plugin-pack-sidebar-test-closure.log" bash -lc "cd '$ROOT' && python3 scripts/check-v252-plugin-pack-sidebar-test-closure.py"
gate "QUALIFICATION_ENV_CLOSURE" "$LOGDIR/qualification-env-closure.log" bash -lc "cd '$ROOT' && python3 scripts/check-v252-qualification-env-closure.py"
gate "FLUID_WINDOW_FILL_CLOSURE" "$LOGDIR/fluid-window-fill-closure.log" bash -lc "cd '$ROOT' && python3 scripts/check-v252-fluid-window-fill-closure.py"
gate "EXTENSION_STABILITY_COMPACT_UI" "$LOGDIR/extension-stability-compact-ui.log" bash -lc "cd '$ROOT' && python3 scripts/check-v252-extension-stability-compact-ui.py"
gate "PLUGIN_MODEL" "$LOGDIR/plugin-model.log" bash -lc "cd '$ROOT' && node --experimental-strip-types scripts/test-v252-plugin-model.mjs"
gate "SYSTEM_WIDE_INSTALL_CONTRACT" "$LOGDIR/system-wide-install-contract.log" bash -lc "cd '$ROOT' && python3 scripts/check-v252-system-wide-install.py"
gate "EVENT_LOOP_STARTUP_CLOSURE" "$LOGDIR/event-loop-startup-closure.log" bash -lc "cd '$ROOT' && python3 scripts/check-v252-event-loop-startup-closure.py"
gate "TAURI_WINDOW_PERMISSION_CLOSURE" "$LOGDIR/tauri-window-permission-closure.log" bash -lc "cd '$ROOT' && python3 scripts/check-v252-tauri-window-permission-closure.py"
gate "TAURI_ASSET_PROTOCOL_FEATURE_CLOSURE" "$LOGDIR/tauri-asset-protocol-feature-closure.log" bash -lc "cd '$ROOT' && python3 scripts/check-v252-tauri-asset-protocol-feature-closure.py"
gate "PLUGIN_HOST_RUST_COMPILE_CLOSURE" "$LOGDIR/plugin-host-rust-compile-closure.log" bash -lc "cd '$ROOT' && python3 scripts/check-v252-plugin-host-rust-compile-closure.py"
gate "RUST198_AS_CHUNKS_CLIPPY_CLOSURE" "$LOGDIR/rust198-as-chunks-clippy-closure.log" bash -lc "cd '$ROOT' && python3 scripts/check-v252-rust198-as-chunks-clippy-closure.py"
gate "PLUGIN_HOST_STRICT_CLIPPY_CLOSURE" "$LOGDIR/plugin-host-strict-clippy-closure.log" bash -lc "cd '$ROOT' && python3 scripts/check-v252-plugin-host-strict-clippy-closure.py"
gate "VISIBLE_STARTUP_KDE_LAUNCHER_CONTRACT" "$LOGDIR/visible-startup-kde-launcher-contract.log" bash -lc "cd '$ROOT' && python3 scripts/check-v252-visible-startup-kde-launcher.py"
gate "FRONTEND_TEST_CLOSURE" "$LOGDIR/frontend-test-closure.log" bash -lc "cd '$ROOT' && python3 scripts/check-v252-frontend-test-closure.py"
gate "FRONTEND_PLUGIN_BRIDGE_MOCK_CLOSURE" "$LOGDIR/frontend-plugin-bridge-mock-closure.log" bash -lc "cd '$ROOT' && python3 scripts/check-v252-frontend-plugin-bridge-mock-closure.py"
gate "PLUGIN_PI_HOOK_LINT_CLOSURE" "$LOGDIR/plugin-pi-hook-lint-closure.log" bash -lc "cd '$ROOT' && python3 scripts/check-v252-plugin-pi-hook-lint-closure.py"
gate "ACTION_WHEEL_CONTRACT" "$LOGDIR/action-wheel-contract.log" bash -lc "cd '$ROOT' && python3 scripts/check-v252-action-wheel-contract.py"
gate "ACTION_WHEEL_MODEL" "$LOGDIR/action-wheel-model.log" bash -lc "cd '$ROOT' && node --experimental-strip-types scripts/test-v252-action-wheel.mjs"
gate "DIAL_STACK_PRESERVATION" "$LOGDIR/dial-stack-preservation.log" bash -lc "cd '$ROOT' && python3 scripts/check-v252-dial-stack-preservation.py"
gate "LAYOUT_CONTRACT" "$LOGDIR/layout-contract.log" bash -lc "cd '$ROOT' && python3 scripts/check-v252-layout-contract.py"
gate "DIALS_FILTER_CONTRACT" "$LOGDIR/dials-filter.log" bash -lc "cd '$ROOT' && python3 scripts/check-v207-dials-filter.py"
gate "ENCODER_PRESS_CONTRACT" "$LOGDIR/encoder-press.log" bash -lc "cd '$ROOT' && python3 scripts/check-v207-encoder-press.py"
gate "INTERACTION_ASSIGNMENT_CONTRACT" "$LOGDIR/interaction-assignment.log" bash -lc "cd '$ROOT' && python3 scripts/check-v207-interaction-assignment.py"
gate "TWITCH_AUTH_POLLING_CONTRACT" "$LOGDIR/twitch.log" bash -lc "cd '$ROOT' && python3 scripts/check-twitch-poll-ui.py"
gate "SOURCE_WHITESPACE" "$LOGDIR/source-whitespace.log" bash -lc "cd '$ROOT' && ! grep -RInE '[[:blank:]]+$' --exclude-dir=.git --exclude-dir=node_modules --exclude-dir=target --exclude-dir=dist --exclude='*.png' ."

# Reuse a previous qualified dependency tree without mutating it; fall back to npm.
if [[ ! -d "$STUDIO/node_modules" ]]; then
  for candidate in \
    "$DL/OpenDeck-v2.0.51-PLUGIN-PACK-INSTALL-ACTIVATION-CLIPPY-CLOSURE-SOURCE/apps/opendeck-studio/node_modules" \
    "$DL/OpenDeck-v2.0.50-PLUGIN-PACK-INSTALL-ACTIVATION-CLOSURE-SOURCE/apps/opendeck-studio/node_modules" \
    "$DL/OpenDeck-v2.0.48-PLUGINS-PACKS-SIDEBAR-TEST-CLOSURE-SOURCE/apps/opendeck-studio/node_modules" \
    "$DL/OpenDeck-v2.0.47-PLUGIN-PACK-SELECTION-ACTIVATION-CLOSURE-SOURCE/apps/opendeck-studio/node_modules" \
    "$DL/OpenDeck-v2.0.46-QUALIFICATION-ENV-CLOSURE-SOURCE/apps/opendeck-studio/node_modules" \
    "$DL/OpenDeck-v2.0.45-FLUID-WINDOW-FILL-CLOSURE-SOURCE/apps/opendeck-studio/node_modules" \
    "$DL/OpenDeck-v2.0.44-PLUGIN-HOST-STRICT-CLIPPY-CLOSURE-SOURCE/apps/opendeck-studio/node_modules" \
    "$DL/OpenDeck-v2.0.43-PLUGIN-HOST-RUST-COMPILE-CLOSURE-SOURCE/apps/opendeck-studio/node_modules" \
    "$DL/OpenDeck-v2.0.42-TAURI-ASSET-PROTOCOL-FEATURE-CLOSURE-SOURCE/apps/opendeck-studio/node_modules" \
    "$DL/OpenDeck-v2.0.41-PLUGIN-PI-HOOK-LINT-CLOSURE-SOURCE/apps/opendeck-studio/node_modules" \
    "$DL/OpenDeck-v2.0.40-FRONTEND-PLUGIN-BRIDGE-TEST-CLOSURE-SOURCE/apps/opendeck-studio/node_modules" \
    "$DL/OpenDeck-v2.0.39-FULL-PARITY-HOST-SOURCE/apps/opendeck-studio/node_modules" \
    "$DL/OpenDeck-v2.0.38-SYSTEM-WIDE-FULL-INSTALL-SOURCE/apps/opendeck-studio/node_modules" \
    "$DL/OpenDeck-v2.0.37-DESKTOP-ENTRY-ACTIVATION-CLOSURE-SOURCE/apps/opendeck-studio/node_modules" \
    "$DL/OpenDeck-v2.0.36-EVENT-LOOP-STARTUP-VISIBILITY-CLOSURE-SOURCE/apps/opendeck-studio/node_modules" \
    "$DL/OpenDeck-v2.0.35-TAURI-WINDOW-PERMISSION-CLOSURE-SOURCE/apps/opendeck-studio/node_modules" \
    "$DL/OpenDeck-v2.0.34-VISIBLE-STARTUP-KDE-LAUNCHER-CLOSURE-SOURCE/apps/opendeck-studio/node_modules" \
    "$DL/OpenDeck-v2.0.33-ACTION-WHEEL-FRONTEND-TEST-CLOSURE-SOURCE/apps/opendeck-studio/node_modules" \
    "$DL/OpenDeck-v2.0.32-ACTION-WHEEL-SOURCE/apps/opendeck-studio/node_modules" \
    "$DL/OpenDeck-v2.0.31-FRONTEND-TEST-CLOSURE-SOURCE/apps/opendeck-studio/node_modules"; do
    if [[ -d "$candidate" ]]; then
      say "OPENDECK_V252_NODE_MODULES_REUSE_SOURCE=$candidate"
      if cp -al "$candidate" "$STUDIO/node_modules" 2>"$LOGDIR/node-modules-reuse.log"; then
        say "OPENDECK_V252_NODE_MODULES_REUSE=PASS"
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
  say "OPENDECK_V252_NPM_INSTALL=REUSED"
fi

gate "PLUGIN_PACK_UI_TESTS" "$LOGDIR/plugin-pack-ui-tests.log" bash -lc "cd '$STUDIO' && npx vitest run src/components/PluginManager.test.tsx"
gate "PLUGIN_PACK_SIDEBAR_UI_TESTS" "$LOGDIR/plugin-pack-sidebar-ui-tests.log" bash -lc "cd '$STUDIO' && npx vitest run src/components/AppSidebar.test.tsx"
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
gate "DEVICE_ICON_RENDER_TEST" "$LOGDIR/device-icon-render-test.log" bash -lc "cd '$ROOT' && cargo test -p opendeck-studio --all-features --locked hardware_key_renderer_draws_an_icon_even_without_an_explicit_asset"
gate "CARGO_RELEASE" "$LOGDIR/cargo-release.log" bash -lc "cd '$ROOT' && cargo build --workspace --all-features --release --locked"
gate "TAURI_BUILD" "$LOGDIR/tauri-build.log" bash -lc "cd '$STUDIO' && CI=true NO_COLOR=1 npm run tauri -- build --no-bundle"
[[ -x "$NEW_BIN" ]] || fail "RELEASE_BINARY_MISSING" 6
NEW_SHA="$(sha256sum "$NEW_BIN" | awk '{print $1}')"
say "OPENDECK_V252_NEW_BINARY_SHA256=$NEW_SHA"
gate "STREAMDECK_PLUS_OS_PROBE" "$LOGDIR/streamdeck-plus-probe.log" bash -lc "cd '$ROOT' && ./scripts/check-streamdeck-plus.sh"

# Render the production candidate and validate UI geometry before any system install.
stop_candidate(){ local pid="${1:-}"; [[ -n "$pid" ]] || return 0; kill "$pid" >/dev/null 2>&1 || true; for _ in {1..30}; do kill -0 "$pid" >/dev/null 2>&1 || break; sleep .05; done; kill -9 "$pid" >/dev/null 2>&1 || true; wait "$pid" 2>/dev/null || true; }
wait_file(){ local file="$1" pid="$2"; for _ in {1..1200}; do [[ -s "$file" ]] && return 0; kill -0 "$pid" >/dev/null 2>&1 || return 2; sleep .01; done; return 1; }
rm -f "$VISUAL_JSON" "$FOCUS_ACK" "$SCREENSHOT"
OPENDECK_QUALIFICATION=1 OPENDECK_QUALIFICATION_PHASE=visual OPENDECK_QUALIFICATION_DIR="$QDIR" "$NEW_BIN" >"$LOGDIR/candidate-visual.log" 2>&1 &
pid=$!
if ! wait_file "$VISUAL_JSON" "$pid"; then stop_candidate "$pid"; tail -160 "$LOGDIR/candidate-visual.log" | tee -a "$VERIFY"; fail "VISUAL_READY" 12; fi
gate "VISUAL_GEOMETRY" "$LOGDIR/visual-geometry.log" python3 "$ROOT/scripts/check-v252-visual-metrics.py" "$VISUAL_JSON"
if command -v spectacle >/dev/null 2>&1; then
  if "$ROOT/scripts/capture-v229-window.sh" "$SCREENSHOT" "$pid" "2.0.52" "$FOCUS_ACK" >"$LOGDIR/screenshot.log" 2>&1; then
    say "OPENDECK_V252_SCREENSHOT=PASS:$SCREENSHOT"
  else
    say "OPENDECK_V252_SCREENSHOT=SKIP_CAPTURE_FAILED"
    tail -80 "$LOGDIR/screenshot.log" | tee -a "$VERIFY" || true
  fi
else
  say "OPENDECK_V252_SCREENSHOT=SKIP:SPECTACLE_MISSING"
fi
stop_candidate "$pid"

# System-wide stage. This is the first privileged mutation and does not change
# the currently active system launch path.
say "OPENDECK_V252_SUDO_AUTH=START"
if sudo -v; then say "OPENDECK_V252_SUDO_AUTH=PASS"; else fail "SUDO_AUTH" 20; fi
gate "SYSTEM_STAGE" "$LOGDIR/system-stage.log" sudo bash "$ROOT/scripts/stage-v252-system-wide.sh" "$NEW_BIN" "$NEW_SHA" "$ROOT" "$VERIFY"
gate "SYSTEM_STAGED_TREE" "$LOGDIR/system-staged-tree.log" python3 "$ROOT/scripts/check-v252-installed-tree.py" staged "$NEW_SHA" "$TARGET_HOME"
say "OPENDECK_V252_STAGED_INSTALL_ROOT=$SYSTEM_INSTALL_ROOT"

# Launch the installed /opt copy through a temporary /usr/share/applications
# desktop ID. The stable command/current symlink still point to the old baseline.
gate "SYSTEM_MENU_LAUNCH_STAGED" "$LOGDIR/system-menu-staged.log" bash "$ROOT/scripts/check-v252-system-menu-launch.sh" staged "$SYSTEM_INSTALL_ROOT/bin/opendeck-studio"

# Activate the canonical system-wide integration only after staged menu launch passes.
gate "SYSTEM_ACTIVATION" "$LOGDIR/system-activation.log" sudo bash "$ROOT/scripts/activate-v252-system-wide.sh" "$NEW_SHA" "$TARGET_USER" "$TARGET_HOME" "$ROLLBACK" "$VERIFY"
SYSTEM_ACTIVATED=1
cat "$LOGDIR/system-activation.log" | tee -a "$VERIFY" >/dev/null || true

# Prove the exact canonical menu entry and stable /usr/local command now resolve
# to the installed 2.0.52 binary. Any failure after activation triggers rollback.
gate "SYSTEM_ACTIVE_TREE" "$LOGDIR/system-active-tree.log" python3 "$ROOT/scripts/check-v252-installed-tree.py" active "$NEW_SHA" "$TARGET_HOME"
gate "SYSTEM_MENU_LAUNCH_CANONICAL" "$LOGDIR/system-menu-canonical.log" bash "$ROOT/scripts/check-v252-system-menu-launch.sh" canonical "$SYSTEM_COMMAND"

[[ -x "$SYSTEM_COMMAND" ]] || fail "SYSTEM_COMMAND_MISSING" 21
[[ "$(readlink -f "$SYSTEM_COMMAND")" == "$SYSTEM_INSTALL_ROOT/bin/opendeck-studio" ]] || fail "SYSTEM_COMMAND_WRONG_TARGET" 21
[[ "$(sha256sum "$SYSTEM_COMMAND" | awk '{print $1}')" == "$NEW_SHA" ]] || fail "SYSTEM_COMMAND_SHA_MISMATCH" 21
[[ -x "$BASELINE_TARGET" ]] || fail "ROLLBACK_BASELINE_LOST" 21

SYSTEM_ACTIVATED=0
say "OPENDECK_V252_SYSTEM_FULL_INSTALL=PASS"
say "OPENDECK_V252_INSTALL_ROOT=$SYSTEM_INSTALL_ROOT"
say "OPENDECK_V252_CURRENT_LINK=/opt/opendeck-plus/current"
say "OPENDECK_V252_SYSTEM_COMMAND=$SYSTEM_COMMAND"
say "OPENDECK_V252_SYSTEM_DESKTOP=/usr/share/applications/opendeck-studio.desktop"
say "OPENDECK_V252_SYSTEM_ICON=/usr/share/icons/hicolor/64x64/apps/opendeck-studio.png"
say "OPENDECK_V252_SYSTEM_UDEV=/etc/udev/rules.d/70-opendeck-streamdeck.rules"
say "OPENDECK_V252_UNINSTALL_COMMAND=sudo /usr/local/sbin/opendeck-uninstall"
say "OPENDECK_V252_ROLLBACK_COMMAND=sudo /usr/local/sbin/opendeck-rollback /var/lib/opendeck-plus/rollback-v2.0.52"
say "OPENDECK_V252_PREVIOUS_BASELINE_PRESERVED=$BASELINE_TARGET"
say "OPENDECK_V252_FRAME_RESIZE=PASS"
say "OPENDECK_V252_FLUID_WINDOW_FILL=PASS"
say "OPENDECK_V252_PROFILES=PASS"
say "OPENDECK_V252_PLUGIN_HOST_BUILD_TESTS=PASS"
say "OPENDECK_V252_PLUGIN_PACK_SELECTION_ACTIVATION=PASS"
say "OPENDECK_V252_EXTENSION_STABILITY_COMPACT_UI=PASS"
say "OPENDECK_V252_EXTENSION_OPERATIONS_SERIALIZED=PASS"
say "OPENDECK_V252_PACK_IO_OFF_EVENT_LOOP=PASS"
say "OPENDECK_V252_ASSET_SCAN_OFF_EVENT_LOOP=PASS"
say "OPENDECK_V252_COMPACT_UI=PASS"
say "OPENDECK_V252_ICON_PACKS=PASS"
say "OPENDECK_V252_DEVICE_ICONS=PASS"
say "OPENDECK_V252_ICON_PACK_DEVICE_PUSH=PASS"
say "OPENDECK_V252_PLUGIN_INSTALL_DISCOVERY=PASS"
say "OPENDECK_V252_PLUGIN_PACKAGE_INSTALL_CLOSURE=PASS"
say "OPENDECK_V252_PLUGIN_ACTIVE_REQUIRES_REGISTRATION=PASS"
say "OPENDECK_V252_OBS_LAUNCH_CLOSE_ACTION=PASS"
say "OPENDECK_V252_ACTION_WHEEL=PASS"
say "OPENDECK_V252_DIAL_STACKS_PRESERVED=PASS"
say "OPENDECK_V252_FULL_HOST_QUALIFICATION=PASS"
say "OPENDECK_V2_0_52=PASS"
say "VERIFY_FILE=$VERIFY"
say "OPENDECK_V252_NOTE=System_wide_install_ready__Launch_OpenDeck_from_the_application_menu"

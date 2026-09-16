#!/usr/bin/env bash
set -euo pipefail

DL="$HOME/Downloads"
ARCHIVE="$DL/OpenDeck-v2.0.8-DIAL-ENCODER-RUSTFMT-CLOSURE-SOURCE.tar.xz"
EXPECTED_ARCHIVE_SHA="4c3039b7b34b6cc64d51289993b5cc2cd7be02884348e173b2dcbacceb9b53b4"
BASE_SOURCE_VERSION="2.0.5"
BASE_SOURCE_SHA="2ca42195a2519fb90383ea91f5c6a29c2655631897317bd63e5c0fa4d7b61e24"
FAILED_V206_SOURCE_SHA="11cdc64bf52c15abd2c2be7c3f25b4f8a38688a987430e69bce9c7a3dd94510e"
FAILED_V207_SOURCE_SHA="7946faa217ee7c4bae58ed3a218364147b43de6af6f6380d1569c2e4f9b87c24"
EXPECTED_V205_SHA="330da7fbd5769de0bd9017c768e06c02c4a97e19ab233b8693d8454e72070602"
SRC="$DL/OpenDeck-v2.0.8-DIAL-ENCODER-RUSTFMT-CLOSURE-SOURCE"
STUDIO="$SRC/apps/opendeck-studio"
INSTALL_ROOT="$HOME/.local/lib/opendeck-v2.0.8"
USER_BIN="$HOME/.local/bin"
VERIFY="$DL/OpenDeck-v2.0.8-DIAL-ENCODER-RUSTFMT-CLOSURE-VERIFY.txt"
ROLLBACK="$DL/OpenDeck-v2.0.8-DIAL-ENCODER-RUSTFMT-CLOSURE-ROLLBACK.txt"
LOGDIR="$DL/OpenDeck-v2.0.8-DIAL-ENCODER-RUSTFMT-CLOSURE-logs"

classify_baseline_sha() {
  case "$1" in
    "$EXPECTED_V205_SHA") printf '%s\n' '2.0.5' ;;
    *) return 1 ;;
  esac
}

if [[ "${1:-}" == "--self-test-baseline-guard" ]]; then
  [[ "$(classify_baseline_sha "$EXPECTED_V205_SHA")" == "2.0.5" ]]
  if classify_baseline_sha "0000000000000000000000000000000000000000000000000000000000000000" >/dev/null 2>&1; then
    printf '%s\n' 'OPENDECK_V208_BASELINE_GUARD_SELF_TEST=FAIL'
    exit 1
  fi
  printf '%s\n' 'OPENDECK_V208_BASELINE_GUARD_SELF_TEST=PASS'
  exit 0
fi

mkdir -p "$LOGDIR" "$USER_BIN"
: > "$VERIFY"

say() { printf '%s\n' "$1" | tee -a "$VERIFY"; }
fail() {
  local stage="$1"
  local rc="${2:-1}"
  say "OPENDECK_V2_0_8_DIAL_ENCODER_RUSTFMT_CLOSURE=FAIL:$rc"
  say "OPENDECK_FAILURE_STAGE=$stage"
  say "VERIFY_FILE=$VERIFY"
  exit "$rc"
}
gate() {
  local name="$1" log="$2"
  shift 2
  say "OPENDECK_V208_STAGE=${name}:START"
  if "$@" >"$log" 2>&1; then
    say "OPENDECK_V208_STAGE=${name}:PASS"
  else
    local rc=$?
    say "OPENDECK_V208_STAGE=${name}:FAIL:$rc"
    tail -240 "$log" | tee -a "$VERIFY"
    fail "$name" "$rc"
  fi
}

say "OPENDECK_V2_0_8_DIAL_ENCODER_RUSTFMT_CLOSURE=START"
say "OPENDECK_RELEASE=DIALS_FILTER_PLUS_ENCODER_PRESS_ROTATION_RUSTFMT_CLOSURE"
say "OPENDECK_BASE_SOURCE_VERSION=$BASE_SOURCE_VERSION"
say "OPENDECK_BASE_SOURCE_SHA256=$BASE_SOURCE_SHA"
say "OPENDECK_CARRIED_FORWARD_FAILED_V206_SOURCE_SHA256=$FAILED_V206_SOURCE_SHA"
say "OPENDECK_CARRIED_FORWARD_FAILED_V207_SOURCE_SHA256=$FAILED_V207_SOURCE_SHA"
say "OPENDECK_V207_FAILURE_STAGE=CARGO_FMT"
say "OPENDECK_REPAIR=RUSTFMT_ENCODER_TEST_LITERAL"
say "OPENDECK_POLICY=QUALIFY_BEFORE_ACTIVE_BINARY_SWITCH"
say "OPENDECK_PRESERVE_CURRENT_QUALIFIED_BASELINE_UNTIL_GREEN=YES"
say "OPENDECK_REQUIRED_ACTIVE_BASELINE=v2.0.5"
say "OPENDECK_FAILED_V206_NEVER_ACCEPTED_AS_ACTIVE_BASELINE=YES"
say "OPENDECK_TWITCH_AUTH_V205_PRESERVE=YES"
say "OPENDECK_WINDOWS_EDITOR_V206_CANDIDATE_CARRIED_FORWARD=YES"
say "OPENDECK_DIAL_PRESS_BINDING=YES"
say "OPENDECK_DIAL_ROTATE_BINDINGS=LEFT,RIGHT"
say "OPENDECK_DIAL_PRESSED_ROTATE_BINDINGS=LEFT,RIGHT"
say "OPENDECK_UDEV_RULES_UNCHANGED=YES"
say "OPENDECK_PUBLIC_STREAM_QUALIFICATION=DISABLED"
say "OPENDECK_PHYSICAL_INPUT_SYNTHESIS=DISABLED"
say "OPENDECK_BACKGROUND_SERVICES=NONE"
say "OPENDECK_AUTOSTART=DISABLED"

for cmd in node npm cargo rustc cargo-clippy rustfmt python3 tar xz sha256sum; do
  command -v "$cmd" >/dev/null 2>&1 || fail "REQUIRED_COMMAND_MISSING:$cmd" 2
done

RUSTC_VERSION="$(rustc --version)"
CARGO_VERSION="$(cargo --version)"
CLIPPY_VERSION="$(cargo clippy --version)"
say "OPENDECK_RUSTC=$RUSTC_VERSION"
say "OPENDECK_CARGO=$CARGO_VERSION"
say "OPENDECK_CLIPPY=$CLIPPY_VERSION"
[[ "$RUSTC_VERSION" == rustc\ 1.98.1* ]] || fail "RUSTC_1_98_1_REQUIRED" 2
[[ "$CARGO_VERSION" == cargo\ 1.98.1* ]] || fail "CARGO_1_98_1_REQUIRED" 2

[[ -x "$USER_BIN/opendeck-studio" ]] || fail "ACTIVE_V205_BASELINE_MISSING:$USER_BIN/opendeck-studio" 2
BASELINE_TARGET="$(readlink -f "$USER_BIN/opendeck-studio" 2>/dev/null || true)"
BASELINE_SHA="$(sha256sum "$USER_BIN/opendeck-studio" | awk '{print $1}')"
if ! BASELINE_VERSION="$(classify_baseline_sha "$BASELINE_SHA")"; then
  say "OPENDECK_ACTIVE_BASELINE_TARGET=$BASELINE_TARGET"
  say "OPENDECK_ACTIVE_BASELINE_BINARY_SHA256=$BASELINE_SHA"
  fail "ACTIVE_V205_BASELINE_SHA_MISMATCH" 2
fi
say "OPENDECK_ACTIVE_BASELINE_VERSION=$BASELINE_VERSION"
say "OPENDECK_ACTIVE_BASELINE_TARGET=$BASELINE_TARGET"
say "OPENDECK_ACTIVE_BASELINE_BINARY_SHA256=$BASELINE_SHA"

[[ -f "$ARCHIVE" ]] || fail "ARCHIVE_MISSING:$ARCHIVE" 3
actual_sha="$(sha256sum "$ARCHIVE" | awk '{print $1}')"
[[ "$actual_sha" == "$EXPECTED_ARCHIVE_SHA" ]] || fail "ARCHIVE_SHA_MISMATCH" 4
say "OPENDECK_SOURCE_SHA256=$actual_sha"

NODE_MODULES_REUSE=""
for candidate in \
  "$DL/OpenDeck-v2.0.7-DIAL-ENCODER-CLOSURE-SOURCE/apps/opendeck-studio/node_modules" \
  "$DL/OpenDeck-v2.0.6-WINDOWS-EDITOR-PARITY-SOURCE/apps/opendeck-studio/node_modules" \
  "$DL/OpenDeck-v2.0.5-TWITCH-AUTH-COMPLETION-SOURCE/apps/opendeck-studio/node_modules" \
  "$STUDIO/node_modules"; do
  if [[ -d "$candidate" ]]; then
    NODE_MODULES_REUSE="$DL/.opendeck-v208-node-modules-reuse-$$"
    rm -rf "$NODE_MODULES_REUSE"
    mv "$candidate" "$NODE_MODULES_REUSE"
    say "OPENDECK_V208_DEPENDENCY_REUSE=CANDIDATE"
    break
  fi
done

rm -rf "$SRC"
mkdir -p "$SRC"
tar -xJf "$ARCHIVE" -C "$SRC"
say "OPENDECK_FRESH_SOURCE_EXTRACT=PASS"

gate "SOURCE_CONTRACT" "$LOGDIR/source-contract.log" \
  bash -lc "cd '$SRC' && python3 scripts/check-clean-baseline.py"
gate "INTERFACE_CONTRACT" "$LOGDIR/interface-contract.log" \
  bash -lc "cd '$SRC' && python3 scripts/check-v207-interface.py"
gate "DIALS_FILTER_CONTRACT" "$LOGDIR/dials-filter-contract.log" \
  bash -lc "cd '$SRC' && python3 scripts/check-v207-dials-filter.py"
gate "ENCODER_PRESS_CONTRACT" "$LOGDIR/encoder-press-contract.log" \
  bash -lc "cd '$SRC' && python3 scripts/check-v207-encoder-press.py"
gate "INTERACTION_ASSIGNMENT_CONTRACT" "$LOGDIR/interaction-assignment-contract.log" \
  bash -lc "cd '$SRC' && python3 scripts/check-v207-interaction-assignment.py"
gate "RUSTFMT_REGRESSION_CONTRACT" "$LOGDIR/rustfmt-regression-contract.log" \
  bash -lc "cd '$SRC' && python3 scripts/check-v208-rustfmt-regression.py"
gate "TWITCH_AUTH_POLLING_CONTRACT" "$LOGDIR/twitch-auth-contract.log" \
  bash -lc "cd '$SRC' && python3 scripts/check-twitch-poll-ui.py"
gate "SOURCE_DIFF_WHITESPACE" "$LOGDIR/source-whitespace.log" \
  bash -lc "cd '$SRC' && ! grep -RInE '[[:blank:]]+$' --exclude-dir=.git --exclude='*.png' ."
gate "VERSION_CONTRACT" "$LOGDIR/version-contract.log" \
  bash -lc "cd '$SRC' && grep -q 'version = \"2.0.8\"' Cargo.toml && grep -q 'version = \"2.0.8\"' apps/opendeck-studio/src-tauri/Cargo.toml && grep -q '\"version\": \"2.0.8\"' apps/opendeck-studio/package.json && grep -q '\"version\": \"2.0.8\"' apps/opendeck-studio/src-tauri/tauri.conf.json && grep -q '# OpenDeck v2.0.8' README.md"
gate "TWITCH_AUTH_UI_CONTRACT" "$LOGDIR/twitch-auth-ui-contract.log" \
  bash -lc "cd '$SRC' && grep -q 'Waiting for Twitch…' apps/opendeck-studio/src/app/App.tsx && grep -q 'Cancel Twitch sign-in' apps/opendeck-studio/src/app/App.tsx && grep -q 'bridge.twitchPollAuth(code.device_code)' apps/opendeck-studio/src/app/App.tsx && grep -q 'expires_in' apps/opendeck-studio/src/app/App.tsx"
gate "OPEN_DEVICE_RESULT_CONTRACT" "$LOGDIR/open-device-result-contract.log" \
  bash -lc "cd '$SRC' && python3 - <<'PY'
from pathlib import Path
s = Path('apps/opendeck-studio/src-tauri/src/streamdeck/runtime.rs').read_text()
assert 'struct OpenedDevice {' in s
assert 'fn open_device() -> Result<Option<OpenedDevice>, String>' in s
assert 'Result<Option<(HidTransport, Option<String>, Option<String>)>, String>' not in s
assert '#[allow(clippy::type_complexity)]' not in s
print('OPENDECK_V208_OPEN_DEVICE_RESULT_REGRESSION=PASS')
PY"
gate "WINDOWS_EDITOR_LAYOUT_CONTRACT" "$LOGDIR/windows-editor-layout-contract.log" \
  bash -lc "cd '$SRC' && grep -q 'className=\"device-surface\"' apps/opendeck-studio/src/components/DeviceEditor.tsx && grep -q 'data-layout-region=\"configuration\"' apps/opendeck-studio/src/components/PropertyInspector.tsx && grep -q 'role=\"tablist\"' apps/opendeck-studio/src/components/ActionLibrary.tsx && grep -q 'aria-label=\"Page options\"' apps/opendeck-studio/src/components/PageNavigator.tsx && ! grep -q '<footer className=\"statusbar\"' apps/opendeck-studio/src/app/App.tsx"
gate "HARDWARE_RUNTIME_CONTRACT" "$LOGDIR/hardware-runtime-contract.log" \
  bash -lc "cd '$SRC' && grep -q 'STREAM_DECK_PLUS_PID: u16 = 0x0084' apps/opendeck-studio/src-tauri/src/streamdeck/protocol.rs && grep -q 'opendeck-streamdeck-plus' apps/opendeck-studio/src-tauri/src/streamdeck/runtime.rs && grep -q 'opendeck://hardware-input' apps/opendeck-studio/src-tauri/src/streamdeck/runtime.rs && grep -q 'pressed: self.dials\[index\]' apps/opendeck-studio/src-tauri/src/streamdeck/runtime.rs && grep -q 'origin === '\''hardware'\''' apps/opendeck-studio/src/app/App.tsx"
gate "UDEV_RULE_CONTRACT" "$LOGDIR/udev-rule-contract.log" \
  bash -lc "cd '$SRC' && grep -q 'ATTR{idProduct}==\"0084\"' packaging/70-opendeck-streamdeck.rules && grep -q 'TAG+=\"uaccess\"' packaging/70-opendeck-streamdeck.rules && ! grep -q 'MODE=' packaging/70-opendeck-streamdeck.rules && bash -n scripts/check-streamdeck-plus.sh"
gate "FRONTEND_HOOK_DEPENDENCY_CONTRACT" "$LOGDIR/frontend-hook-dependency-contract.log" \
  bash -lc "cd '$SRC' && python3 - <<'PY'
from pathlib import Path
s = Path('apps/opendeck-studio/src/app/App.tsx').read_text()
assert 'workspace: typeof state.workspace' not in s
assert 'workspace: Workspace' in s
assert 'type Workspace,' in s
print('OPENDECK_V208_CALLBACK_DEPENDENCY_REGRESSION=PASS')
PY"

if [[ -n "$NODE_MODULES_REUSE" && -d "$NODE_MODULES_REUSE" ]]; then
  mv "$NODE_MODULES_REUSE" "$STUDIO/node_modules"
  say "OPENDECK_V208_DEPENDENCY_REUSE=PASS"
  say "OPENDECK_V208_NPM_INSTALL=REUSED_PREVIOUS_PASS"
else
  gate "NPM_INSTALL" "$LOGDIR/npm-install.log" \
    bash -lc "cd '$STUDIO' && npm install --prefer-offline --no-audit --no-fund"
fi

gate "FRONTEND_TESTS" "$LOGDIR/frontend-tests.log" \
  bash -lc "cd '$STUDIO' && npm test"
gate "FRONTEND_LINT" "$LOGDIR/frontend-lint.log" \
  bash -lc "cd '$STUDIO' && npm run lint"
gate "FRONTEND_BUILD" "$LOGDIR/frontend-build.log" \
  bash -lc "cd '$STUDIO' && npm run build"

gate "CARGO_LOCK" "$LOGDIR/cargo-lock.log" \
  bash -lc "cd '$SRC' && cargo generate-lockfile"
gate "CARGO_FETCH" "$LOGDIR/cargo-fetch.log" \
  bash -lc "cd '$SRC' && cargo fetch --locked"
gate "CARGO_FMT" "$LOGDIR/cargo-fmt.log" \
  bash -lc "cd '$SRC' && cargo fmt --all -- --check"
gate "CARGO_CHECK" "$LOGDIR/cargo-check.log" \
  bash -lc "cd '$SRC' && cargo check --workspace --all-targets --all-features --locked"
gate "CARGO_CLIPPY_STRICT" "$LOGDIR/cargo-clippy.log" \
  bash -lc "cd '$SRC' && cargo clippy --workspace --all-targets --all-features --locked -- -D warnings"
gate "CARGO_TEST" "$LOGDIR/cargo-test.log" \
  bash -lc "cd '$SRC' && cargo test --workspace --all-targets --all-features --locked"
gate "CARGO_RELEASE" "$LOGDIR/cargo-release.log" \
  bash -lc "cd '$SRC' && cargo build --workspace --all-features --release --locked"
gate "TAURI_BUILD" "$LOGDIR/tauri-build.log" \
  bash -lc "cd '$STUDIO' && CI=true NO_COLOR=1 npm run tauri -- build --no-bundle"
gate "STREAMDECK_PLUS_OS_PROBE" "$LOGDIR/streamdeck-plus-probe.log" \
  bash -lc "cd '$SRC' && ./scripts/check-streamdeck-plus.sh"

NEW_BIN="$SRC/target/release/opendeck-studio"
[[ -x "$NEW_BIN" ]] || fail "NEW_BINARY_MISSING:$NEW_BIN" 6
NEW_SHA="$(sha256sum "$NEW_BIN" | awk '{print $1}')"
say "OPENDECK_V208_NEW_BINARY_SHA256=$NEW_SHA"
say "OPENDECK_V208_NEW_TREE_QUALIFIED=PASS"
say "OPENDECK_V208_UDEV_INSTALL=SKIPPED_UNCHANGED_FROM_V205"

PREV_TARGET="$BASELINE_TARGET"
{
  printf 'PREVIOUS_OPENDECK_STUDIO_TARGET=%s\n' "$PREV_TARGET"
  printf 'PREVIOUS_OPENDECK_STUDIO_SHA256=%s\n' "$BASELINE_SHA"
  printf "RESTORE_COMMAND=ln -sfn '%s' '%s'\n" "$PREV_TARGET" "$USER_BIN/opendeck-studio"
} > "$ROLLBACK"
chmod 600 "$ROLLBACK"

STAGE="$HOME/.local/lib/.opendeck-v2.0.8-stage-$$"
rm -rf "$STAGE"
mkdir -p "$STAGE/bin"
install -m 0755 "$NEW_BIN" "$STAGE/bin/opendeck-studio"
STAGED_SHA="$(sha256sum "$STAGE/bin/opendeck-studio" | awk '{print $1}')"
[[ "$STAGED_SHA" == "$NEW_SHA" ]] || fail "STAGED_BINARY_SHA_MISMATCH" 8

if [[ -d "$INSTALL_ROOT" ]]; then
  old_root="$HOME/.local/lib/opendeck-v2.0.8-previous-$(date +%Y%m%d-%H%M%S)"
  mv "$INSTALL_ROOT" "$old_root"
  say "OPENDECK_PREVIOUS_V208_ROOT=$old_root"
fi
mv "$STAGE" "$INSTALL_ROOT"
ln -sfn "$INSTALL_ROOT/bin/opendeck-studio" "$USER_BIN/opendeck-studio"
ln -sfn "$INSTALL_ROOT/bin/opendeck-studio" "$USER_BIN/opendeck"

ACTIVE_SHA="$(sha256sum "$USER_BIN/opendeck-studio" | awk '{print $1}')"
[[ "$ACTIVE_SHA" == "$NEW_SHA" ]] || fail "ACTIVE_BINARY_SHA_MISMATCH" 9

service_count=0
if [[ -d "$HOME/.config/systemd/user" ]]; then
  service_count="$(find "$HOME/.config/systemd/user" -maxdepth 1 -type f -iname '*opendeck*.service' | wc -l)"
fi
autostart_count=0
if [[ -d "$HOME/.config/autostart" ]]; then
  autostart_count="$(find "$HOME/.config/autostart" -maxdepth 1 -type f -iname '*opendeck*.desktop' | wc -l)"
fi
[[ "$service_count" == "0" ]] || fail "BACKGROUND_SERVICE_FOUND" 10
[[ "$autostart_count" == "0" ]] || fail "AUTOSTART_ENTRY_FOUND" 10

say "OPENDECK_V208_INSTALL_ROOT=$INSTALL_ROOT"
say "OPENDECK_V208_ACTIVE_BINARY_SHA256=$ACTIVE_SHA"
say "OPENDECK_V208_BACKGROUND_SERVICES=$service_count"
say "OPENDECK_V208_AUTOSTART_ENTRIES=$autostart_count"
say "OPENDECK_V208_STREAMDECK_PLUS_OS_PROBE=PASS"
say "OPENDECK_V2_0_8_DIAL_ENCODER_RUSTFMT_CLOSURE_QUALIFY=PASS"
say "OPENDECK_V2_0_8_DIAL_ENCODER_RUSTFMT_CLOSURE_ACTIVATE=PASS"
say "OPENDECK_V2_0_8_DIAL_ENCODER_RUSTFMT_CLOSURE=PASS"
say "VERIFY_FILE=$VERIFY"
say "ROLLBACK_FILE=$ROLLBACK"
say "RUN_COMMAND=$USER_BIN/opendeck-studio"

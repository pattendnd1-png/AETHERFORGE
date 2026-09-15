#!/usr/bin/env bash
set -euo pipefail

DL="$HOME/Downloads"
ARCHIVE="$DL/OpenDeck-v2.0.5-TWITCH-AUTH-COMPLETION-SOURCE.tar.xz"
EXPECTED_ARCHIVE_SHA="2ca42195a2519fb90383ea91f5c6a29c2655631897317bd63e5c0fa4d7b61e24"
BASE_SOURCE_VERSION="2.0.4"
BASE_SOURCE_SHA="e4dd0be00a6fecf6af8f15dd26755a695b84c86ddc35c736a2c51ba43b18a7a7"
EXPECTED_V204_SHA="7f3dd6fd0be4b4d99c7e7b80f325b7fd5efa9ef242d61737c9fa568f11add9c7"
PREVIOUS_SRC="$DL/OpenDeck-v2.0.4-WINDOWS-HARDWARE-SOURCE"
SRC="$DL/OpenDeck-v2.0.5-TWITCH-AUTH-COMPLETION-SOURCE"
STUDIO="$SRC/apps/opendeck-studio"
INSTALL_ROOT="$HOME/.local/lib/opendeck-v2.0.5"
USER_BIN="$HOME/.local/bin"
VERIFY="$DL/OpenDeck-v2.0.5-TWITCH-AUTH-COMPLETION-VERIFY.txt"
ROLLBACK="$DL/OpenDeck-v2.0.5-TWITCH-AUTH-COMPLETION-ROLLBACK.txt"
LOGDIR="$DL/OpenDeck-v2.0.5-TWITCH-AUTH-COMPLETION-logs"
INSTALL_UDEV="${OPENDECK_INSTALL_UDEV:-0}"

classify_baseline_sha() {
  case "$1" in
    "$EXPECTED_V204_SHA") printf '%s\n' '2.0.4' ;;
    *) return 1 ;;
  esac
}

if [[ "${1:-}" == "--self-test-baseline-guard" ]]; then
  [[ "$(classify_baseline_sha "$EXPECTED_V204_SHA")" == "2.0.4" ]]
  if classify_baseline_sha "0000000000000000000000000000000000000000000000000000000000000000" >/dev/null 2>&1; then
    printf '%s\n' 'OPENDECK_V205_BASELINE_GUARD_SELF_TEST=FAIL'
    exit 1
  fi
  printf '%s\n' 'OPENDECK_V205_BASELINE_GUARD_SELF_TEST=PASS'
  exit 0
fi

mkdir -p "$LOGDIR" "$USER_BIN"
: > "$VERIFY"

say() { printf '%s\n' "$1" | tee -a "$VERIFY"; }
fail() {
  local stage="$1"
  local rc="${2:-1}"
  say "OPENDECK_V2_0_5_TWITCH_AUTH=FAIL:$rc"
  say "OPENDECK_FAILURE_STAGE=$stage"
  say "VERIFY_FILE=$VERIFY"
  exit "$rc"
}
gate() {
  local name="$1" log="$2"
  shift 2
  say "OPENDECK_V205_STAGE=${name}:START"
  if "$@" >"$log" 2>&1; then
    say "OPENDECK_V205_STAGE=${name}:PASS"
  else
    local rc=$?
    say "OPENDECK_V205_STAGE=${name}:FAIL:$rc"
    tail -220 "$log" | tee -a "$VERIFY"
    fail "$name" "$rc"
  fi
}

say "OPENDECK_V2_0_5_TWITCH_AUTH=START"
say "OPENDECK_RELEASE=TWITCH_DEVICE_AUTH_POLLING_COMPLETION"
say "OPENDECK_BASE_SOURCE_VERSION=$BASE_SOURCE_VERSION"
say "OPENDECK_BASE_SOURCE_SHA256=$BASE_SOURCE_SHA"
say "OPENDECK_REPAIR=TWITCH_DEVICE_FLOW_POLL_CANCEL_EXPIRY"
say "OPENDECK_POLICY=QUALIFY_BEFORE_ACTIVE_BINARY_SWITCH"
say "OPENDECK_PRESERVE_CURRENT_QUALIFIED_BASELINE_UNTIL_GREEN=YES"
say "OPENDECK_REQUIRED_ACTIVE_BASELINE=v2.0.4"
say "OPENDECK_INTERFACE_REBUILD=NOT_INCLUDED_IN_V2.0.5"
say "OPENDECK_PUBLIC_STREAM_QUALIFICATION=DISABLED"
say "OPENDECK_PHYSICAL_INPUT_SYNTHESIS=DISABLED"
say "OPENDECK_BACKGROUND_SERVICES=NONE"
say "OPENDECK_AUTOSTART=DISABLED"
say "OPENDECK_UDEV_INSTALL_REQUESTED=$INSTALL_UDEV"

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

[[ -x "$USER_BIN/opendeck-studio" ]] || fail "ACTIVE_V204_BASELINE_MISSING:$USER_BIN/opendeck-studio" 2
BASELINE_TARGET="$(readlink -f "$USER_BIN/opendeck-studio" 2>/dev/null || true)"
BASELINE_SHA="$(sha256sum "$USER_BIN/opendeck-studio" | awk '{print $1}')"
if ! BASELINE_VERSION="$(classify_baseline_sha "$BASELINE_SHA")"; then
  say "OPENDECK_ACTIVE_BASELINE_TARGET=$BASELINE_TARGET"
  say "OPENDECK_ACTIVE_BASELINE_BINARY_SHA256=$BASELINE_SHA"
  fail "ACTIVE_V204_BASELINE_SHA_MISMATCH" 2
fi
say "OPENDECK_ACTIVE_BASELINE_VERSION=$BASELINE_VERSION"
say "OPENDECK_ACTIVE_BASELINE_TARGET=$BASELINE_TARGET"
say "OPENDECK_ACTIVE_BASELINE_BINARY_SHA256=$BASELINE_SHA"

[[ -f "$ARCHIVE" ]] || fail "ARCHIVE_MISSING:$ARCHIVE" 3
actual_sha="$(sha256sum "$ARCHIVE" | awk '{print $1}')"
[[ "$actual_sha" == "$EXPECTED_ARCHIVE_SHA" ]] || fail "ARCHIVE_SHA_MISMATCH" 4
say "OPENDECK_SOURCE_SHA256=$actual_sha"

NODE_MODULES_REUSE=""
if [[ -d "$PREVIOUS_SRC/apps/opendeck-studio/node_modules" ]]; then
  NODE_MODULES_REUSE="$DL/.opendeck-v205-node-modules-reuse-$$"
  rm -rf "$NODE_MODULES_REUSE"
  mv "$PREVIOUS_SRC/apps/opendeck-studio/node_modules" "$NODE_MODULES_REUSE"
  say "OPENDECK_V205_DEPENDENCY_REUSE=CANDIDATE"
elif [[ -d "$STUDIO/node_modules" ]]; then
  NODE_MODULES_REUSE="$DL/.opendeck-v205-node-modules-reuse-$$"
  rm -rf "$NODE_MODULES_REUSE"
  mv "$STUDIO/node_modules" "$NODE_MODULES_REUSE"
  say "OPENDECK_V205_DEPENDENCY_REUSE=CANDIDATE"
fi

rm -rf "$SRC"
mkdir -p "$SRC"
tar -xJf "$ARCHIVE" -C "$SRC" --strip-components=1
say "OPENDECK_FRESH_SOURCE_EXTRACT=PASS"

gate "SOURCE_CONTRACT" "$LOGDIR/source-contract.log" \
  bash -lc "cd '$SRC' && python3 scripts/check-clean-baseline.py"
gate "TWITCH_AUTH_POLLING_CONTRACT" "$LOGDIR/twitch-auth-contract.log" \
  bash -lc "cd '$SRC' && python3 scripts/check-twitch-poll-ui.py"
gate "SOURCE_DIFF_WHITESPACE" "$LOGDIR/source-whitespace.log" \
  bash -lc "cd '$SRC' && ! grep -RInE '[[:blank:]]+$' --exclude-dir=.git --exclude='*.png' ."
gate "VERSION_CONTRACT" "$LOGDIR/version-contract.log" \
  bash -lc "cd '$SRC' && grep -q 'version = \"2.0.5\"' Cargo.toml && grep -q 'version = \"2.0.5\"' apps/opendeck-studio/src-tauri/Cargo.toml && grep -q '\"version\": \"2.0.5\"' apps/opendeck-studio/package.json && grep -q '\"version\": \"2.0.5\"' apps/opendeck-studio/src-tauri/tauri.conf.json && grep -q 'OpenDeck 2.0.5' apps/opendeck-studio/src/app/App.tsx"
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
print('OPENDECK_V205_OPEN_DEVICE_RESULT_REGRESSION=PASS')
PY"
gate "WINDOWS_SHELL_CONTRACT" "$LOGDIR/windows-shell-contract.log" \
  bash -lc "cd '$SRC' && grep -q 'grid-template-areas:\"editor divider actions\"' apps/opendeck-studio/src/styles.css && grep -q 'data-layout-region=\"actions\"' apps/opendeck-studio/src/components/ActionLibrary.tsx && grep -q 'data-layout-region=\"inspector\"' apps/opendeck-studio/src/components/PropertyInspector.tsx"
gate "HARDWARE_RUNTIME_CONTRACT" "$LOGDIR/hardware-runtime-contract.log" \
  bash -lc "cd '$SRC' && grep -q 'STREAM_DECK_PLUS_PID: u16 = 0x0084' apps/opendeck-studio/src-tauri/src/streamdeck/protocol.rs && grep -q 'opendeck-streamdeck-plus' apps/opendeck-studio/src-tauri/src/streamdeck/runtime.rs && grep -q 'opendeck://hardware-input' apps/opendeck-studio/src-tauri/src/streamdeck/runtime.rs && grep -q 'origin === '\''hardware'\''' apps/opendeck-studio/src/app/App.tsx"
gate "UDEV_RULE_CONTRACT" "$LOGDIR/udev-rule-contract.log" \
  bash -lc "cd '$SRC' && grep -q 'ATTR{idProduct}==\"0084\"' packaging/70-opendeck-streamdeck.rules && grep -q 'TAG+=\"uaccess\"' packaging/70-opendeck-streamdeck.rules && ! grep -q 'MODE=' packaging/70-opendeck-streamdeck.rules && bash -n scripts/check-streamdeck-plus.sh"
gate "FRONTEND_HOOK_DEPENDENCY_CONTRACT" "$LOGDIR/frontend-hook-dependency-contract.log" \
  bash -lc "cd '$SRC' && python3 - <<'PY'
from pathlib import Path
s = Path('apps/opendeck-studio/src/app/App.tsx').read_text()
assert 'workspace: typeof state.workspace' not in s
assert 'workspace: Workspace' in s
assert 'type Workspace,' in s
print('OPENDECK_V205_CALLBACK_DEPENDENCY_REGRESSION=PASS')
PY"

if [[ -n "$NODE_MODULES_REUSE" && -d "$NODE_MODULES_REUSE" ]]; then
  mv "$NODE_MODULES_REUSE" "$STUDIO/node_modules"
  say "OPENDECK_V205_DEPENDENCY_REUSE=PASS"
  say "OPENDECK_V205_NPM_INSTALL=REUSED_PREVIOUS_PASS"
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

NEW_BIN="$SRC/target/release/opendeck-studio"
[[ -x "$NEW_BIN" ]] || fail "NEW_BINARY_MISSING:$NEW_BIN" 6
NEW_SHA="$(sha256sum "$NEW_BIN" | awk '{print $1}')"
say "OPENDECK_V205_NEW_BINARY_SHA256=$NEW_SHA"
say "OPENDECK_V205_NEW_TREE_QUALIFIED=PASS"

if [[ "$INSTALL_UDEV" == "1" ]]; then
  command -v sudo >/dev/null 2>&1 || fail "SUDO_REQUIRED_FOR_UDEV_INSTALL" 7
  command -v udevadm >/dev/null 2>&1 || fail "UDEVADM_REQUIRED_FOR_UDEV_INSTALL" 7
  say "OPENDECK_V205_UDEV_INSTALL=START"
  if sudo install -Dm644 "$SRC/packaging/70-opendeck-streamdeck.rules" /etc/udev/rules.d/70-opendeck-streamdeck.rules \
    && sudo udevadm control --reload-rules \
    && sudo udevadm trigger --subsystem-match=usb --action=change \
    && sudo udevadm trigger --subsystem-match=hidraw --action=change; then
    say "OPENDECK_V205_UDEV_INSTALL=PASS"
  else
    fail "UDEV_INSTALL" 7
  fi
else
  say "OPENDECK_V205_UDEV_INSTALL=SKIPPED_UNCHANGED_FROM_V204"
fi

PREV_TARGET="$BASELINE_TARGET"
{
  printf 'PREVIOUS_OPENDECK_STUDIO_TARGET=%s\n' "$PREV_TARGET"
  printf 'PREVIOUS_OPENDECK_STUDIO_SHA256=%s\n' "$BASELINE_SHA"
  printf "RESTORE_COMMAND=ln -sfn '%s' '%s'\n" "$PREV_TARGET" "$USER_BIN/opendeck-studio"
} > "$ROLLBACK"
chmod 600 "$ROLLBACK"

STAGE="$HOME/.local/lib/.opendeck-v2.0.5-stage-$$"
rm -rf "$STAGE"
mkdir -p "$STAGE/bin"
install -m 0755 "$NEW_BIN" "$STAGE/bin/opendeck-studio"
STAGED_SHA="$(sha256sum "$STAGE/bin/opendeck-studio" | awk '{print $1}')"
[[ "$STAGED_SHA" == "$NEW_SHA" ]] || fail "STAGED_BINARY_SHA_MISMATCH" 8

if [[ -d "$INSTALL_ROOT" ]]; then
  old_root="$HOME/.local/lib/opendeck-v2.0.5-previous-$(date +%Y%m%d-%H%M%S)"
  mv "$INSTALL_ROOT" "$old_root"
  say "OPENDECK_PREVIOUS_V205_ROOT=$old_root"
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

say "OPENDECK_V205_INSTALL_ROOT=$INSTALL_ROOT"
say "OPENDECK_V205_ACTIVE_BINARY_SHA256=$ACTIVE_SHA"
say "OPENDECK_V205_BACKGROUND_SERVICES=$service_count"
say "OPENDECK_V205_AUTOSTART_ENTRIES=$autostart_count"

if "$SRC/scripts/check-streamdeck-plus.sh" >"$LOGDIR/streamdeck-plus-probe.log" 2>&1; then
  say "OPENDECK_V205_STREAMDECK_PLUS_OS_PROBE=PASS"
else
  probe_rc=$?
  say "OPENDECK_V205_STREAMDECK_PLUS_OS_PROBE=PENDING:$probe_rc"
  tail -40 "$LOGDIR/streamdeck-plus-probe.log" | tee -a "$VERIFY"
  say "OPENDECK_V205_PHYSICAL_ACCEPTANCE=PENDING"
fi

say "OPENDECK_V2_0_5_TWITCH_AUTH_QUALIFY=PASS"
say "OPENDECK_V2_0_5_TWITCH_AUTH_ACTIVATE=PASS"
say "OPENDECK_V2_0_5_TWITCH_AUTH=PASS"
say "VERIFY_FILE=$VERIFY"
say "ROLLBACK_FILE=$ROLLBACK"
say "RUN_COMMAND=$USER_BIN/opendeck-studio"

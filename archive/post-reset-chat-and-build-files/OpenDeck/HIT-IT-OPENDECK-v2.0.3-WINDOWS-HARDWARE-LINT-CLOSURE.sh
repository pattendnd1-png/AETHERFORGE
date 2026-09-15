#!/usr/bin/env bash
set -euo pipefail

DL="$HOME/Downloads"
ARCHIVE="$DL/OpenDeck-v2.0.3-WINDOWS-HARDWARE-LINT-CLOSURE-SOURCE.tar.xz"
EXPECTED_ARCHIVE_SHA="ce72b42d964dca2587cf5a046ed29f57f3bc80626c1dfacedc94219c5c7bb066"
EXPECTED_SOURCE_COMMIT="e68ffab86edcf33f128b3754a167224840ad842b"
EXPECTED_V201_SHA="39c1e7dfe0c219efadff0e4ee684878d328b370b772a81d6abf1aa7b48c70503"
EXPECTED_V202_SHA="ec7594e2d6bde90b9cc2ee0c43c7e03ed263346b23bbec681e385c43c8089037"
SRC="$DL/OpenDeck-v2.0.3-WINDOWS-HARDWARE-SOURCE"
STUDIO="$SRC/apps/opendeck-studio"
INSTALL_ROOT="$HOME/.local/lib/opendeck-v2.0.3"
USER_BIN="$HOME/.local/bin"
VERIFY="$DL/OpenDeck-v2.0.3-WINDOWS-HARDWARE-LINT-CLOSURE-VERIFY.txt"
ROLLBACK="$DL/OpenDeck-v2.0.3-WINDOWS-HARDWARE-LINT-CLOSURE-ROLLBACK.txt"
LOGDIR="$DL/OpenDeck-v2.0.3-WINDOWS-HARDWARE-LINT-CLOSURE-logs"
INSTALL_UDEV="${OPENDECK_INSTALL_UDEV:-0}"

classify_baseline_sha() {
  case "$1" in
    "$EXPECTED_V201_SHA") printf '%s\n' '2.0.1' ;;
    "$EXPECTED_V202_SHA") printf '%s\n' '2.0.2' ;;
    *) return 1 ;;
  esac
}

if [[ "${1:-}" == "--self-test-baseline-guard" ]]; then
  [[ "$(classify_baseline_sha "$EXPECTED_V201_SHA")" == "2.0.1" ]]
  [[ "$(classify_baseline_sha "$EXPECTED_V202_SHA")" == "2.0.2" ]]
  if classify_baseline_sha "0000000000000000000000000000000000000000000000000000000000000000" >/dev/null 2>&1; then
    printf '%s\n' 'OPENDECK_V203_BASELINE_GUARD_SELF_TEST=FAIL'
    exit 1
  fi
  printf '%s\n' 'OPENDECK_V203_BASELINE_GUARD_SELF_TEST=PASS'
  exit 0
fi

mkdir -p "$LOGDIR" "$USER_BIN"
: > "$VERIFY"

say() { printf '%s\n' "$1" | tee -a "$VERIFY"; }
fail() {
  local stage="$1"
  local rc="${2:-1}"
  say "OPENDECK_V2_0_3_WINDOWS_HARDWARE=FAIL:$rc"
  say "OPENDECK_FAILURE_STAGE=$stage"
  say "VERIFY_FILE=$VERIFY"
  exit "$rc"
}
gate() {
  local name="$1" log="$2"
  shift 2
  say "OPENDECK_V203_STAGE=${name}:START"
  if "$@" >"$log" 2>&1; then
    say "OPENDECK_V203_STAGE=${name}:PASS"
  else
    local rc=$?
    say "OPENDECK_V203_STAGE=${name}:FAIL:$rc"
    tail -220 "$log" | tee -a "$VERIFY"
    fail "$name" "$rc"
  fi
}

say "OPENDECK_V2_0_3_WINDOWS_HARDWARE=START"
say "OPENDECK_REPAIR=REACT_HOOK_CALLBACK_TYPE_DEPENDENCY_CLOSURE"
say "OPENDECK_RELEASE=WINDOWS_STYLE_SHELL_PLUS_STREAM_DECK_PLUS_HID_RUNTIME"
say "OPENDECK_POLICY=QUALIFY_BEFORE_ACTIVE_BINARY_SWITCH"
say "OPENDECK_PRESERVE_CURRENT_QUALIFIED_BASELINE_UNTIL_GREEN=YES"
say "OPENDECK_ACCEPTED_BASELINES=v2.0.1,v2.0.2"
say "OPENDECK_SOURCE_COMMIT=$EXPECTED_SOURCE_COMMIT"
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

# Preserve the exact currently active, previously qualified OpenDeck baseline until
# every v2.0.3 software gate is green. Accept only the two host-qualified hashes
# from this release line; any unknown binary still hard-fails before extraction/build.
[[ -x "$USER_BIN/opendeck-studio" ]] || fail "ACTIVE_QUALIFIED_BASELINE_MISSING:$USER_BIN/opendeck-studio" 2
BASELINE_TARGET="$(readlink -f "$USER_BIN/opendeck-studio" 2>/dev/null || true)"
BASELINE_SHA="$(sha256sum "$USER_BIN/opendeck-studio" | awk '{print $1}')"
if ! BASELINE_VERSION="$(classify_baseline_sha "$BASELINE_SHA")"; then
  say "OPENDECK_ACTIVE_BASELINE_TARGET=$BASELINE_TARGET"
  say "OPENDECK_ACTIVE_BASELINE_BINARY_SHA256=$BASELINE_SHA"
  fail "ACTIVE_QUALIFIED_BASELINE_SHA_MISMATCH" 2
fi
say "OPENDECK_ACTIVE_BASELINE_VERSION=$BASELINE_VERSION"
say "OPENDECK_ACTIVE_BASELINE_TARGET=$BASELINE_TARGET"
say "OPENDECK_ACTIVE_BASELINE_BINARY_SHA256=$BASELINE_SHA"

[[ -f "$ARCHIVE" ]] || fail "ARCHIVE_MISSING:$ARCHIVE" 3
actual_sha="$(sha256sum "$ARCHIVE" | awk '{print $1}')"
[[ "$actual_sha" == "$EXPECTED_ARCHIVE_SHA" ]] || fail "ARCHIVE_SHA_MISMATCH" 4
say "OPENDECK_SOURCE_SHA256=$actual_sha"

NODE_MODULES_REUSE=""
if [[ -d "$STUDIO/node_modules" ]]; then
  NODE_MODULES_REUSE="$DL/.opendeck-v203-node-modules-reuse-$$"
  rm -rf "$NODE_MODULES_REUSE"
  mv "$STUDIO/node_modules" "$NODE_MODULES_REUSE"
  say "OPENDECK_V203_DEPENDENCY_REUSE=CANDIDATE"
fi

rm -rf "$SRC"
mkdir -p "$SRC"
tar -xJf "$ARCHIVE" -C "$SRC" --strip-components=1
say "OPENDECK_FRESH_SOURCE_EXTRACT=PASS"

gate "SOURCE_CONTRACT" "$LOGDIR/source-contract.log" \
  bash -lc "cd '$SRC' && python3 scripts/check-clean-baseline.py"
gate "SOURCE_DIFF_WHITESPACE" "$LOGDIR/source-whitespace.log" \
  bash -lc "cd '$SRC' && ! grep -RInE '[[:blank:]]+$' --exclude-dir=.git --exclude='*.png' ."
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
print('OPENDECK_V203_CALLBACK_DEPENDENCY_REGRESSION=PASS')
PY"

# Reuse the dependency tree from the immediately preceding v2.0.3 frontend run when
# available. Source/package metadata are unchanged by this closure; only App.tsx changed.
if [[ -n "$NODE_MODULES_REUSE" && -d "$NODE_MODULES_REUSE" ]]; then
  mv "$NODE_MODULES_REUSE" "$STUDIO/node_modules"
  say "OPENDECK_V203_DEPENDENCY_REUSE=PASS"
  say "OPENDECK_V203_NPM_INSTALL=REUSED_PREVIOUS_PASS"
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

# Resolve exactly one dependency graph for all Rust/Tauri gates.
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
say "OPENDECK_V203_NEW_BINARY_SHA256=$NEW_SHA"
say "OPENDECK_V203_NEW_TREE_QUALIFIED=PASS"

# Optional, explicitly requested system integration. The recommended one-line handoff sets
# OPENDECK_INSTALL_UDEV=1, so sudo escalation is visible and intentional rather than silent.
if [[ "$INSTALL_UDEV" == "1" ]]; then
  command -v sudo >/dev/null 2>&1 || fail "SUDO_REQUIRED_FOR_UDEV_INSTALL" 7
  command -v udevadm >/dev/null 2>&1 || fail "UDEVADM_REQUIRED_FOR_UDEV_INSTALL" 7
  say "OPENDECK_V203_UDEV_INSTALL=START"
  if sudo install -Dm644 "$SRC/packaging/70-opendeck-streamdeck.rules" /etc/udev/rules.d/70-opendeck-streamdeck.rules \
    && sudo udevadm control --reload-rules \
    && sudo udevadm trigger --subsystem-match=usb --action=change \
    && sudo udevadm trigger --subsystem-match=hidraw --action=change; then
    say "OPENDECK_V203_UDEV_INSTALL=PASS"
  else
    fail "UDEV_INSTALL" 7
  fi
else
  say "OPENDECK_V203_UDEV_INSTALL=SKIPPED"
  say "OPENDECK_V203_UDEV_INSTALL_COMMAND=sudo install -Dm644 '$SRC/packaging/70-opendeck-streamdeck.rules' /etc/udev/rules.d/70-opendeck-streamdeck.rules && sudo udevadm control --reload-rules && sudo udevadm trigger --subsystem-match=hidraw --action=change"
fi

PREV_TARGET="$BASELINE_TARGET"
{
  printf 'PREVIOUS_OPENDECK_STUDIO_TARGET=%s\n' "$PREV_TARGET"
  printf 'PREVIOUS_OPENDECK_STUDIO_SHA256=%s\n' "$BASELINE_SHA"
  printf "RESTORE_COMMAND=ln -sfn '%s' '%s'\n" "$PREV_TARGET" "$USER_BIN/opendeck-studio"
} > "$ROLLBACK"
chmod 600 "$ROLLBACK"

STAGE="$HOME/.local/lib/.opendeck-v2.0.3-stage-$$"
rm -rf "$STAGE"
mkdir -p "$STAGE/bin"
install -m 0755 "$NEW_BIN" "$STAGE/bin/opendeck-studio"
STAGED_SHA="$(sha256sum "$STAGE/bin/opendeck-studio" | awk '{print $1}')"
[[ "$STAGED_SHA" == "$NEW_SHA" ]] || fail "STAGED_BINARY_SHA_MISMATCH" 8

if [[ -d "$INSTALL_ROOT" ]]; then
  old_root="$HOME/.local/lib/opendeck-v2.0.3-previous-$(date +%Y%m%d-%H%M%S)"
  mv "$INSTALL_ROOT" "$old_root"
  say "OPENDECK_PREVIOUS_V203_ROOT=$old_root"
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

say "OPENDECK_V203_INSTALL_ROOT=$INSTALL_ROOT"
say "OPENDECK_V203_ACTIVE_BINARY_SHA256=$ACTIVE_SHA"
say "OPENDECK_V203_BACKGROUND_SERVICES=$service_count"
say "OPENDECK_V203_AUTOSTART_ENTRIES=$autostart_count"

# Physical hardware is a separate acceptance gate. Probe only; never synthesize input/output here.
if "$SRC/scripts/check-streamdeck-plus.sh" >"$LOGDIR/streamdeck-plus-probe.log" 2>&1; then
  say "OPENDECK_V203_STREAMDECK_PLUS_OS_PROBE=PASS"
else
  probe_rc=$?
  say "OPENDECK_V203_STREAMDECK_PLUS_OS_PROBE=PENDING:$probe_rc"
  tail -40 "$LOGDIR/streamdeck-plus-probe.log" | tee -a "$VERIFY"
  say "OPENDECK_V203_PHYSICAL_ACCEPTANCE=PENDING"
fi

say "OPENDECK_V2_0_3_WINDOWS_HARDWARE_QUALIFY=PASS"
say "OPENDECK_V2_0_3_WINDOWS_HARDWARE_ACTIVATE=PASS"
say "OPENDECK_V2_0_3_WINDOWS_HARDWARE=PASS"
say "VERIFY_FILE=$VERIFY"
say "ROLLBACK_FILE=$ROLLBACK"
say "RUN_COMMAND=$USER_BIN/opendeck-studio"

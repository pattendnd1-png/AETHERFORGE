#!/usr/bin/env bash
set -euo pipefail

DL="$HOME/Downloads"
ARCHIVE="$DL/OpenDeck-v2.0.2-ACTION-RUNTIME-SOURCE.tar.xz"
EXPECTED_ARCHIVE_SHA="fb2b6d68d146f1d0ee935a08540fcdc8cc9a6dd9c84ea8fdc5b7de3a0edbd9c6"
EXPECTED_SOURCE_COMMIT="cf19318008d95df97a5469db21747f2233fe6181"
SRC="$DL/OpenDeck-v2.0.2-ACTION-RUNTIME-SOURCE"
STUDIO="$SRC/apps/opendeck-studio"
INSTALL_ROOT="$HOME/.local/lib/opendeck-v2.0.2"
USER_BIN="$HOME/.local/bin"
VERIFY="$DL/OpenDeck-v2.0.2-ACTION-RUNTIME-VERIFY.txt"
ROLLBACK="$DL/OpenDeck-v2.0.2-ACTION-RUNTIME-ROLLBACK.txt"
LOGDIR="$DL/OpenDeck-v2.0.2-ACTION-RUNTIME-logs"

mkdir -p "$LOGDIR" "$USER_BIN"
: > "$VERIFY"

say() { printf '%s\n' "$1" | tee -a "$VERIFY"; }
fail() {
  local stage="$1"
  local rc="${2:-1}"
  say "OPENDECK_V2_0_2_ACTION_RUNTIME=FAIL:$rc"
  say "OPENDECK_FAILURE_STAGE=$stage"
  say "VERIFY_FILE=$VERIFY"
  exit "$rc"
}
gate() {
  local name="$1" log="$2"
  shift 2
  say "OPENDECK_V202_STAGE=${name}:START"
  if "$@" >"$log" 2>&1; then
    say "OPENDECK_V202_STAGE=${name}:PASS"
  else
    local rc=$?
    say "OPENDECK_V202_STAGE=${name}:FAIL:$rc"
    tail -180 "$log" | tee -a "$VERIFY"
    fail "$name" "$rc"
  fi
}

say "OPENDECK_V2_0_2_ACTION_RUNTIME=START"
say "OPENDECK_REPAIR=EXPLICIT_TEST_ACTION_PLUS_EDITOR_NATIVE_ACTION_EXECUTION"
say "OPENDECK_POLICY=QUALIFY_BEFORE_ACTIVE_BINARY_SWITCH"
say "OPENDECK_STABLE_V2_0_1_PRESERVE_UNTIL_GREEN=YES"
say "OPENDECK_SOURCE_COMMIT=$EXPECTED_SOURCE_COMMIT"
say "OPENDECK_PUBLIC_STREAM_QUALIFICATION=DISABLED"
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

[[ -f "$ARCHIVE" ]] || fail "ARCHIVE_MISSING:$ARCHIVE" 3
actual_sha="$(sha256sum "$ARCHIVE" | awk '{print $1}')"
[[ "$actual_sha" == "$EXPECTED_ARCHIVE_SHA" ]] || fail "ARCHIVE_SHA_MISMATCH" 4
say "OPENDECK_SOURCE_SHA256=$actual_sha"

rm -rf "$SRC"
mkdir -p "$SRC"
tar -xJf "$ARCHIVE" -C "$SRC" --strip-components=1
say "OPENDECK_FRESH_SOURCE_EXTRACT=PASS"

gate "SOURCE_CONTRACT" "$LOGDIR/source-contract.log" \
  bash -lc "cd '$SRC' && python3 scripts/check-clean-baseline.py"
gate "SOURCE_DIFF_WHITESPACE" "$LOGDIR/source-whitespace.log" \
  bash -lc "cd '$SRC' && ! grep -RInE '[[:blank:]]+$' --exclude-dir=.git --exclude='*.png' ."
gate "ACTION_RUNTIME_CONTRACT" "$LOGDIR/action-runtime-contract.log" \
  bash -lc "cd '$SRC' && grep -q 'resolveActionExecution' apps/opendeck-studio/src/app/action-executor.ts && grep -q '>Test Action<' apps/opendeck-studio/src/inspector/ActionInspector.tsx && grep -q 'Toggle OBS streaming now?' apps/opendeck-studio/src/app/App.tsx && grep -q 'Toggle OBS recording now?' apps/opendeck-studio/src/app/App.tsx"

# Frontend changes are new in v2.0.2, so all frontend gates are rerun fresh.
gate "NPM_INSTALL" "$LOGDIR/npm-install.log" \
  bash -lc "cd '$STUDIO' && npm install --prefer-offline --no-audit --no-fund"
gate "FRONTEND_TESTS" "$LOGDIR/frontend-tests.log" \
  bash -lc "cd '$STUDIO' && npm test"
gate "FRONTEND_LINT" "$LOGDIR/frontend-lint.log" \
  bash -lc "cd '$STUDIO' && npm run lint"
gate "FRONTEND_BUILD" "$LOGDIR/frontend-build.log" \
  bash -lc "cd '$STUDIO' && npm run build"

# Native gates use one exact lockfile generated for this source tree.
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
say "OPENDECK_V202_NEW_BINARY_SHA256=$NEW_SHA"
say "OPENDECK_V202_NEW_TREE_QUALIFIED=PASS"

PREV_TARGET=""
if [[ -e "$USER_BIN/opendeck-studio" || -L "$USER_BIN/opendeck-studio" ]]; then
  PREV_TARGET="$(readlink -f "$USER_BIN/opendeck-studio" 2>/dev/null || true)"
fi
{
  printf 'PREVIOUS_OPENDECK_STUDIO_TARGET=%s\n' "$PREV_TARGET"
  if [[ -n "$PREV_TARGET" ]]; then
    printf "RESTORE_COMMAND=ln -sfn '%s' '%s'\n" "$PREV_TARGET" "$USER_BIN/opendeck-studio"
  fi
} > "$ROLLBACK"
chmod 600 "$ROLLBACK"

STAGE="$HOME/.local/lib/.opendeck-v2.0.2-stage-$$"
rm -rf "$STAGE"
mkdir -p "$STAGE/bin"
install -m 0755 "$NEW_BIN" "$STAGE/bin/opendeck-studio"
STAGED_SHA="$(sha256sum "$STAGE/bin/opendeck-studio" | awk '{print $1}')"
[[ "$STAGED_SHA" == "$NEW_SHA" ]] || fail "STAGED_BINARY_SHA_MISMATCH" 7

if [[ -d "$INSTALL_ROOT" ]]; then
  old_root="$HOME/.local/lib/opendeck-v2.0.2-previous-$(date +%Y%m%d-%H%M%S)"
  mv "$INSTALL_ROOT" "$old_root"
  say "OPENDECK_PREVIOUS_V202_ROOT=$old_root"
fi
mv "$STAGE" "$INSTALL_ROOT"
ln -sfn "$INSTALL_ROOT/bin/opendeck-studio" "$USER_BIN/opendeck-studio"
ln -sfn "$INSTALL_ROOT/bin/opendeck-studio" "$USER_BIN/opendeck"

ACTIVE_SHA="$(sha256sum "$USER_BIN/opendeck-studio" | awk '{print $1}')"
[[ "$ACTIVE_SHA" == "$NEW_SHA" ]] || fail "ACTIVE_BINARY_SHA_MISMATCH" 8

say "OPENDECK_V202_INSTALL_ROOT=$INSTALL_ROOT"
say "OPENDECK_V202_ACTIVE_BINARY_SHA256=$ACTIVE_SHA"
say "OPENDECK_V202_BACKGROUND_SERVICES=0"
say "OPENDECK_V202_AUTOSTART=DISABLED"
say "OPENDECK_V2_0_2_ACTION_RUNTIME_QUALIFY=PASS"
say "OPENDECK_V2_0_2_ACTION_RUNTIME_ACTIVATE=PASS"
say "OPENDECK_V2_0_2_ACTION_RUNTIME=PASS"
say "VERIFY_FILE=$VERIFY"
say "ROLLBACK_FILE=$ROLLBACK"
say "RUN_COMMAND=$USER_BIN/opendeck-studio"

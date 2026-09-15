#!/usr/bin/env bash
set -euo pipefail

DL="$HOME/Downloads"
ARCHIVE="$DL/OpenDeck-v2.0.1-CUSTOMIZATION-HOST-GATE-CLOSURE-SOURCE.tar.xz"
EXPECTED_ARCHIVE_SHA="d4b8e4ef480bbdbc3f4afd730c04407911de3be9f004a9b864bf06184883c197"
EXPECTED_SOURCE_COMMIT="cb25ca9d5a695eca77915a7bd8c1dddf9496c7a6"
SRC="$DL/OpenDeck-v2.0.1-CUSTOMIZATION-HOST-GATE-CLOSURE-SOURCE"
STUDIO="$SRC/apps/opendeck-studio"
INSTALL_ROOT="$HOME/.local/lib/opendeck-v2.0.1"
USER_BIN="$HOME/.local/bin"
VERIFY="$DL/OpenDeck-v2.0.1-CUSTOMIZATION-HOST-GATE-CLOSURE-VERIFY.txt"
ROLLBACK="$DL/OpenDeck-v2.0.1-CUSTOMIZATION-HOST-GATE-CLOSURE-ROLLBACK.txt"
LOGDIR="$DL/OpenDeck-v2.0.1-CUSTOMIZATION-HOST-GATE-CLOSURE-logs"
PREVIOUS_VERIFY="$DL/OpenDeck-v2.0.1-CUSTOMIZATION-VERIFY.txt"

mkdir -p "$LOGDIR" "$USER_BIN"
: > "$VERIFY"

say() { printf '%s\n' "$1" | tee -a "$VERIFY"; }
fail() {
  local stage="$1"
  local rc="${2:-1}"
  say "OPENDECK_V2_0_1_CUSTOMIZATION_HOST_GATE_CLOSURE=FAIL:$rc"
  say "OPENDECK_FAILURE_STAGE=$stage"
  say "VERIFY_FILE=$VERIFY"
  exit "$rc"
}
gate() {
  local name="$1" log="$2"
  shift 2
  say "OPENDECK_V201_STAGE=${name}:START"
  if "$@" >"$log" 2>&1; then
    say "OPENDECK_V201_STAGE=${name}:PASS"
  else
    local rc=$?
    say "OPENDECK_V201_STAGE=${name}:FAIL:$rc"
    tail -160 "$log" | tee -a "$VERIFY"
    fail "$name" "$rc"
  fi
}

say "OPENDECK_V2_0_1_CUSTOMIZATION_HOST_GATE_CLOSURE=START"
say "OPENDECK_REPAIR=MARKDOWN_HARD_BREAK_WHITESPACE_NORMALIZATION"
say "OPENDECK_POLICY=QUALIFY_BEFORE_ACTIVE_BINARY_SWITCH"
say "OPENDECK_STABLE_V2_0_0_PRESERVE_UNTIL_GREEN=YES"
say "OPENDECK_SOURCE_COMMIT=$EXPECTED_SOURCE_COMMIT"
say "OPENDECK_BACKGROUND_SERVICES=NONE"
say "OPENDECK_AUTOSTART=DISABLED"

if [[ -f "$PREVIOUS_VERIFY" ]] && grep -Fqx 'OPENDECK_FAILURE_STAGE=SOURCE_DIFF_WHITESPACE' "$PREVIOUS_VERIFY"; then
  say "OPENDECK_PREVIOUS_WHITESPACE_FAILURE_EVIDENCE=PASS"
else
  say "OPENDECK_PREVIOUS_WHITESPACE_FAILURE_EVIDENCE=NOT_REQUIRED"
fi

for cmd in node npm cargo rustc python3 tar xz sha256sum; do
  command -v "$cmd" >/dev/null 2>&1 || fail "REQUIRED_COMMAND_MISSING:$cmd" 2
done

[[ -f "$ARCHIVE" ]] || fail "ARCHIVE_MISSING:$ARCHIVE" 3
actual_sha="$(sha256sum "$ARCHIVE" | awk '{print $1}')"
[[ "$actual_sha" == "$EXPECTED_ARCHIVE_SHA" ]] || fail "ARCHIVE_SHA_MISMATCH" 4
say "OPENDECK_SOURCE_SHA256=$actual_sha"

rm -rf "$SRC"
mkdir -p "$SRC"
tar -xJf "$ARCHIVE" -C "$SRC" --strip-components=1
say "OPENDECK_FRESH_SOURCE_EXTRACT=PASS"

# Run the exact source gates that previously diverged between sandbox packaging and host qualification.
gate "SOURCE_CONTRACT" "$LOGDIR/source-contract.log" bash -lc "cd '$SRC' && python3 scripts/check-clean-baseline.py"
gate "SOURCE_DIFF_WHITESPACE" "$LOGDIR/source-whitespace.log" bash -lc "cd '$SRC' && ! grep -RInE '[[:blank:]]+$' --exclude-dir=.git --exclude='*.png' ."

# Frontend gate.
gate "NPM_INSTALL" "$LOGDIR/npm-install.log" bash -lc "cd '$STUDIO' && npm install --prefer-offline --no-audit --no-fund"
gate "FRONTEND_TESTS" "$LOGDIR/frontend-tests.log" bash -lc "cd '$STUDIO' && npm test"
gate "FRONTEND_LINT" "$LOGDIR/frontend-lint.log" bash -lc "cd '$STUDIO' && npm run lint"
gate "FRONTEND_BUILD" "$LOGDIR/frontend-build.log" bash -lc "cd '$STUDIO' && npm run build"

# Native gate.
gate "CARGO_LOCK" "$LOGDIR/cargo-lock.log" bash -lc "cd '$SRC' && cargo generate-lockfile"
gate "CARGO_FETCH" "$LOGDIR/cargo-fetch.log" bash -lc "cd '$SRC' && cargo fetch --locked"
gate "CARGO_FMT" "$LOGDIR/cargo-fmt.log" bash -lc "cd '$SRC' && cargo fmt --all -- --check"
gate "CARGO_CHECK" "$LOGDIR/cargo-check.log" bash -lc "cd '$SRC' && cargo check --workspace --all-targets --all-features --locked"
gate "CARGO_CLIPPY_STRICT" "$LOGDIR/cargo-clippy.log" bash -lc "cd '$SRC' && cargo clippy --workspace --all-targets --all-features --locked -- -D warnings"
gate "CARGO_TEST" "$LOGDIR/cargo-test.log" bash -lc "cd '$SRC' && cargo test --workspace --all-targets --all-features --locked"
gate "CARGO_RELEASE" "$LOGDIR/cargo-release.log" bash -lc "cd '$SRC' && cargo build --workspace --all-features --release --locked"
gate "TAURI_BUILD" "$LOGDIR/tauri-build.log" bash -lc "cd '$STUDIO' && CI=true NO_COLOR=1 npm run tauri -- build --no-bundle"

NEW_BIN="$SRC/target/release/opendeck-studio"
[[ -x "$NEW_BIN" ]] || fail "NEW_BINARY_MISSING:$NEW_BIN" 6
NEW_SHA="$(sha256sum "$NEW_BIN" | awk '{print $1}')"
say "OPENDECK_V201_NEW_BINARY_SHA256=$NEW_SHA"
say "OPENDECK_V201_NEW_TREE_QUALIFIED=PASS"

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

STAGE="$HOME/.local/lib/.opendeck-v2.0.1-stage-$$"
rm -rf "$STAGE"
mkdir -p "$STAGE/bin"
install -m 0755 "$NEW_BIN" "$STAGE/bin/opendeck-studio"
STAGED_SHA="$(sha256sum "$STAGE/bin/opendeck-studio" | awk '{print $1}')"
[[ "$STAGED_SHA" == "$NEW_SHA" ]] || fail "STAGED_BINARY_SHA_MISMATCH" 7

if [[ -d "$INSTALL_ROOT" ]]; then
  old_root="$HOME/.local/lib/opendeck-v2.0.1-previous-$(date +%Y%m%d-%H%M%S)"
  mv "$INSTALL_ROOT" "$old_root"
  say "OPENDECK_PREVIOUS_V201_ROOT=$old_root"
fi
mv "$STAGE" "$INSTALL_ROOT"
ln -sfn "$INSTALL_ROOT/bin/opendeck-studio" "$USER_BIN/opendeck-studio"
ln -sfn "$INSTALL_ROOT/bin/opendeck-studio" "$USER_BIN/opendeck"

ACTIVE_SHA="$(sha256sum "$USER_BIN/opendeck-studio" | awk '{print $1}')"
[[ "$ACTIVE_SHA" == "$NEW_SHA" ]] || fail "ACTIVE_BINARY_SHA_MISMATCH" 8

say "OPENDECK_V201_INSTALL_ROOT=$INSTALL_ROOT"
say "OPENDECK_V201_ACTIVE_BINARY_SHA256=$ACTIVE_SHA"
say "OPENDECK_V201_BACKGROUND_SERVICES=0"
say "OPENDECK_V201_AUTOSTART=DISABLED"
say "OPENDECK_V2_0_1_CUSTOMIZATION_QUALIFY=PASS"
say "OPENDECK_V2_0_1_CUSTOMIZATION_ACTIVATE=PASS"
say "OPENDECK_V2_0_1_CUSTOMIZATION_HOST_GATE_CLOSURE=PASS"
say "VERIFY_FILE=$VERIFY"
say "ROLLBACK_FILE=$ROLLBACK"
say "RUN_COMMAND=$USER_BIN/opendeck-studio"

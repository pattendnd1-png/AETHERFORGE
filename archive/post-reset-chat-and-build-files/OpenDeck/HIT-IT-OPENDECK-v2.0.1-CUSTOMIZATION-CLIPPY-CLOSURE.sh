#!/usr/bin/env bash
set -euo pipefail

DL="$HOME/Downloads"
ARCHIVE="$DL/OpenDeck-v2.0.1-CUSTOMIZATION-CLIPPY-CLOSURE-SOURCE.tar.xz"
EXPECTED_ARCHIVE_SHA="7dfd1a1d5db22bbddac6419adf86740b72d93973ee4b562751b299189a516d07"
EXPECTED_SOURCE_COMMIT="aa2d81a35912f9510e8bfe51e753ec752e3ab9fd"
PREVIOUS_VERIFY="$DL/OpenDeck-v2.0.1-CUSTOMIZATION-LINT-CLOSURE-VERIFY.txt"
REUSE_SRC="$DL/OpenDeck-v2.0.1-CUSTOMIZATION-HOST-GATE-CLOSURE-SOURCE"
FRESH_SRC="$DL/OpenDeck-v2.0.1-CUSTOMIZATION-CLIPPY-CLOSURE-SOURCE"
CHECK_SRC="$DL/.opendeck-v201-clippy-closure-check-$$"
INSTALL_ROOT="$HOME/.local/lib/opendeck-v2.0.1"
USER_BIN="$HOME/.local/bin"
VERIFY="$DL/OpenDeck-v2.0.1-CUSTOMIZATION-CLIPPY-CLOSURE-VERIFY.txt"
ROLLBACK="$DL/OpenDeck-v2.0.1-CUSTOMIZATION-CLIPPY-CLOSURE-ROLLBACK.txt"
LOGDIR="$DL/OpenDeck-v2.0.1-CUSTOMIZATION-CLIPPY-CLOSURE-logs"

mkdir -p "$LOGDIR" "$USER_BIN"
: > "$VERIFY"

say() { printf '%s\n' "$1" | tee -a "$VERIFY"; }
fail() {
  local stage="$1"
  local rc="${2:-1}"
  say "OPENDECK_V2_0_1_CUSTOMIZATION_CLIPPY_CLOSURE=FAIL:$rc"
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
    tail -220 "$log" | tee -a "$VERIFY"
    fail "$name" "$rc"
  fi
}
cleanup() { rm -rf "$CHECK_SRC"; }
trap cleanup EXIT

say "OPENDECK_V2_0_1_CUSTOMIZATION_CLIPPY_CLOSURE=START"
say "OPENDECK_REPAIR=STRICT_CLIPPY_FIVE_FINDING_STRUCTURAL_CLOSURE"
say "OPENDECK_POLICY=QUALIFY_BEFORE_ACTIVE_BINARY_SWITCH"
say "OPENDECK_STABLE_V2_0_0_PRESERVE_UNTIL_GREEN=YES"
say "OPENDECK_SOURCE_COMMIT=$EXPECTED_SOURCE_COMMIT"
say "OPENDECK_BACKGROUND_SERVICES=NONE"
say "OPENDECK_AUTOSTART=DISABLED"

for cmd in node npm cargo rustc python3 tar xz sha256sum grep; do
  command -v "$cmd" >/dev/null 2>&1 || fail "REQUIRED_COMMAND_MISSING:$cmd" 2
done

# Fail closed unless this is resuming the exact strict-Clippy failure reported by the prior run.
if [[ -f "$PREVIOUS_VERIFY" ]] \
  && grep -Fqx 'OPENDECK_V201_STAGE=FRONTEND_TESTS:PASS' "$PREVIOUS_VERIFY" \
  && grep -Fqx 'OPENDECK_V201_STAGE=FRONTEND_LINT:PASS' "$PREVIOUS_VERIFY" \
  && grep -Fqx 'OPENDECK_V201_STAGE=FRONTEND_BUILD:PASS' "$PREVIOUS_VERIFY" \
  && grep -Fqx 'OPENDECK_V201_STAGE=CARGO_CHECK:PASS' "$PREVIOUS_VERIFY" \
  && grep -Fqx 'OPENDECK_V201_STAGE=CARGO_CLIPPY_STRICT:FAIL:101' "$PREVIOUS_VERIFY" \
  && grep -Fqx 'OPENDECK_FAILURE_STAGE=CARGO_CLIPPY_STRICT' "$PREVIOUS_VERIFY"; then
  say "OPENDECK_PREVIOUS_CLIPPY_FAILURE_EVIDENCE=PASS"
else
  say "OPENDECK_PREVIOUS_CLIPPY_FAILURE_EVIDENCE=UNAVAILABLE"
fi

[[ -f "$ARCHIVE" ]] || fail "ARCHIVE_MISSING:$ARCHIVE" 3
actual_sha="$(sha256sum "$ARCHIVE" | awk '{print $1}')"
[[ "$actual_sha" == "$EXPECTED_ARCHIVE_SHA" ]] || fail "ARCHIVE_SHA_MISMATCH" 4
say "OPENDECK_SOURCE_SHA256=$actual_sha"

# Qualify the canonical source archive before touching any reusable host tree.
rm -rf "$CHECK_SRC"
mkdir -p "$CHECK_SRC"
tar -xJf "$ARCHIVE" -C "$CHECK_SRC" --strip-components=1
CHECK_STUDIO="$CHECK_SRC/apps/opendeck-studio"
gate "SOURCE_CONTRACT" "$LOGDIR/source-contract.log" bash -lc "cd '$CHECK_SRC' && python3 scripts/check-clean-baseline.py"
gate "SOURCE_DIFF_WHITESPACE" "$LOGDIR/source-whitespace.log" bash -lc "cd '$CHECK_SRC' && ! grep -RInE '[[:blank:]]+$' --exclude-dir=.git --exclude='*.png' ."

# Exact source-shape regression for every strict-Clippy finding from the host log.
gate "CLIPPY_REPAIR_CONTRACT" "$LOGDIR/clippy-repair-contract.log" python3 - "$CHECK_SRC" <<'PY'
from pathlib import Path
import sys
root = Path(sys.argv[1])
editor = (root / 'apps/opendeck-studio/src-tauri/src/editor.rs').read_text()
assets = (root / 'apps/opendeck-studio/src-tauri/src/assets.rs').read_text()
lib = (root / 'apps/opendeck-studio/src-tauri/src/lib.rs').read_text()
checks = {
    'dead_action_instance_removed': 'struct ActionInstance' not in editor,
    'asset_sort_by_key': 'assets.sort_by_key(|asset| asset.name.to_lowercase());' in assets,
    'workspace_backup_nested_if_removed': 'if primary.exists() {\n        if let Ok(text)' not in editor,
    'browser_open_nested_if_removed': 'if let Ok(status) = Command::new(program).args(args).status() {\n            if status.success()' not in lib,
    'tests_after_run': lib.find('pub fn run()') < lib.rfind('#[cfg(test)]\nmod tests'),
    'no_clippy_suppression': '#[allow(clippy::' not in editor + assets + lib and '#[expect(clippy::' not in editor + assets + lib,
    'no_dead_code_suppression': '#[allow(dead_code)]' not in editor + assets + lib and '#[expect(dead_code)]' not in editor + assets + lib,
}
for name, ok in checks.items():
    print(f'OPENDECK_CLIPPY_REPAIR_{name.upper()}={"PASS" if ok else "FAIL"}')
if not all(checks.values()):
    raise SystemExit(1)
PY
say "OPENDECK_V201_CLIPPY_STRUCTURAL_CLOSURE=PASS"

# Reuse the host tree that already passed npm install/tests/lint/build and Cargo fetch/check.
if [[ -f "$REUSE_SRC/apps/opendeck-studio/package.json" \
   && -x "$REUSE_SRC/apps/opendeck-studio/node_modules/.bin/vite" \
   && -f "$REUSE_SRC/Cargo.lock" ]]; then
  SRC="$REUSE_SRC"
  STUDIO="$SRC/apps/opendeck-studio"
  tar -xJf "$ARCHIVE" -C "$SRC" --strip-components=1
  say "OPENDECK_V201_SOURCE_MODE=REUSE_PREVIOUS_GREEN_TREE"
  say "OPENDECK_V201_FRONTEND_GATES=REUSED_PREVIOUS_PASS"
  say "OPENDECK_V201_CARGO_FETCH=REUSED_PREVIOUS_PASS"
else
  SRC="$FRESH_SRC"
  STUDIO="$SRC/apps/opendeck-studio"
  rm -rf "$SRC"
  mkdir -p "$SRC"
  tar -xJf "$ARCHIVE" -C "$SRC" --strip-components=1
  say "OPENDECK_V201_SOURCE_MODE=FRESH_SOURCE_FALLBACK"
  gate "NPM_INSTALL" "$LOGDIR/npm-install.log" bash -lc "cd '$STUDIO' && npm install --prefer-offline --no-audit --no-fund"
  gate "FRONTEND_TESTS" "$LOGDIR/frontend-tests.log" bash -lc "cd '$STUDIO' && npm test"
  gate "FRONTEND_LINT" "$LOGDIR/frontend-lint.log" bash -lc "cd '$STUDIO' && npm run lint"
  gate "FRONTEND_BUILD" "$LOGDIR/frontend-build.log" bash -lc "cd '$STUDIO' && npm run build"
  gate "CARGO_LOCK" "$LOGDIR/cargo-lock.log" bash -lc "cd '$SRC' && cargo generate-lockfile"
  gate "CARGO_FETCH" "$LOGDIR/cargo-fetch.log" bash -lc "cd '$SRC' && cargo fetch --locked"
fi

# Rust production code changed, so native gates restart at fmt/check rather than trusting the previous check.
gate "CARGO_FMT" "$LOGDIR/cargo-fmt.log" bash -lc "cd '$SRC' && cargo fmt --all -- --check"
gate "CARGO_CHECK" "$LOGDIR/cargo-check.log" bash -lc "cd '$SRC' && cargo check --workspace --all-targets --all-features --locked"
gate "CARGO_CLIPPY_STRICT" "$LOGDIR/cargo-clippy.log" bash -lc "cd '$SRC' && cargo clippy --workspace --all-targets --all-features --locked -- -D warnings"
gate "CARGO_TEST" "$LOGDIR/cargo-test.log" bash -lc "cd '$SRC' && cargo test --workspace --all-targets --all-features --locked"
gate "CARGO_RELEASE" "$LOGDIR/cargo-release.log" bash -lc "cd '$SRC' && cargo build --workspace --all-features --release --locked"
# Tauri's beforeBuildCommand re-runs the frontend production build against the repaired tree.
gate "TAURI_BUILD" "$LOGDIR/tauri-build.log" bash -lc "cd '$STUDIO' && CI=true NO_COLOR=1 npm run tauri -- build --no-bundle"

NEW_BIN="$SRC/target/release/opendeck-studio"
[[ -x "$NEW_BIN" ]] || fail "NEW_BINARY_MISSING:$NEW_BIN" 7
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
[[ "$STAGED_SHA" == "$NEW_SHA" ]] || fail "STAGED_BINARY_SHA_MISMATCH" 8

if [[ -d "$INSTALL_ROOT" ]]; then
  old_root="$HOME/.local/lib/opendeck-v2.0.1-previous-$(date +%Y%m%d-%H%M%S)"
  mv "$INSTALL_ROOT" "$old_root"
  say "OPENDECK_PREVIOUS_V201_ROOT=$old_root"
fi
mv "$STAGE" "$INSTALL_ROOT"
ln -sfn "$INSTALL_ROOT/bin/opendeck-studio" "$USER_BIN/opendeck-studio"
ln -sfn "$INSTALL_ROOT/bin/opendeck-studio" "$USER_BIN/opendeck"

ACTIVE_SHA="$(sha256sum "$USER_BIN/opendeck-studio" | awk '{print $1}')"
[[ "$ACTIVE_SHA" == "$NEW_SHA" ]] || fail "ACTIVE_BINARY_SHA_MISMATCH" 9

say "OPENDECK_V201_INSTALL_ROOT=$INSTALL_ROOT"
say "OPENDECK_V201_ACTIVE_BINARY_SHA256=$ACTIVE_SHA"
say "OPENDECK_V201_BACKGROUND_SERVICES=0"
say "OPENDECK_V201_AUTOSTART=DISABLED"
say "OPENDECK_V2_0_1_CUSTOMIZATION_QUALIFY=PASS"
say "OPENDECK_V2_0_1_CUSTOMIZATION_ACTIVATE=PASS"
say "OPENDECK_V2_0_1_CUSTOMIZATION_CLIPPY_CLOSURE=PASS"
say "VERIFY_FILE=$VERIFY"
say "ROLLBACK_FILE=$ROLLBACK"
say "RUN_COMMAND=$USER_BIN/opendeck-studio"

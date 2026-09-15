#!/usr/bin/env bash
set -euo pipefail

GENERATION="20260913-WINDOWS-SHELL-CHECKPOINT-2"
ARCHIVE="OpenDeck-v1.3.0-WINDOWS-PARITY-SOURCE-CHECKPOINT.tar.xz"
EXPECTED_SHA256="b070dafb6e4b3c40d78096a0046c53133c53bc7a66f15fe2403e9d29e1529b5e"
DOWNLOADS="${HOME}/Downloads"
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
WORK_ROOT="${DOWNLOADS}/.opendeck-v130-windows-parity-checkpoint"
TMP_ROOT="${WORK_ROOT}/tmp"
LOG="${DOWNLOADS}/OpenDeck-v1.3.0-WINDOWS-SHELL-CHECKPOINT-HOST.log"
VERIFY="${DOWNLOADS}/OpenDeck-v1.3.0-WINDOWS-SHELL-CHECKPOINT-HOST-VERIFY.txt"
CLIPPY_LOG="${DOWNLOADS}/OpenDeck-v1.3.0-CLIPPY-CLOSURE.log"
CLEAN_ARCHIVE="${DOWNLOADS}/OpenDeck-v1.3.0-WINDOWS-SHELL-HOST-CLEAN-SOURCE.tar.xz"
CLEAN_SHA="${DOWNLOADS}/OpenDeck-v1.3.0-WINDOWS-SHELL-HOST-CLEAN-SOURCE.sha256"
PREVIEW_LOG="${DOWNLOADS}/OpenDeck-v1.3.0-WINDOWS-SHELL-PREVIEW.log"
SUCCESS=0

mark() {
  printf '%s\n' "$1"
  printf '%s\n' "$1" >> "$VERIFY"
}

on_exit() {
  local rc=$?
  if (( SUCCESS == 0 )); then
    printf 'OPENDECK_V1_3_0_WINDOWS_SHELL_CHECKPOINT=FAIL:%s\n' "$rc" | tee -a "$VERIFY"
    printf 'OPENDECK_V1_3_0_TERMINAL_SESSION=PRESERVED\n' | tee -a "$VERIFY"
  fi
}
trap on_exit EXIT

if [[ "${1:-}" == "--self-test" ]]; then
  [[ "$GENERATION" == "20260913-WINDOWS-SHELL-CHECKPOINT-2" ]]
  [[ -d "$SCRIPT_DIR" ]]
  [[ "$EXPECTED_SHA256" =~ ^[0-9a-f]{64}$ ]]
  [[ "$WORK_ROOT" == "$HOME/Downloads/"* ]]
  [[ "$TMP_ROOT" == "$HOME/Downloads/"* ]]
  echo "OPENDECK_V130_RUNNER_SELF_TEST=PASS"
  SUCCESS=1
  trap - EXIT
  exit 0
fi

mkdir -p "$DOWNLOADS"
: > "$LOG"
: > "$VERIFY"
exec > >(tee -a "$LOG") 2>&1

mark "OPENDECK_V130_CHECKPOINT_GENERATION=${GENERATION}"
mark "OPENDECK_V130_RELEASE_STATUS=DEVELOPMENT_CHECKPOINT_NOT_CANONICAL_RELEASE"
mark "OPENDECK_V130_SOURCE_ARCHIVE=${ARCHIVE}"

ARCHIVE_PATH="${SCRIPT_DIR}/${ARCHIVE}"
[[ -f "$ARCHIVE_PATH" ]] || { echo "ERROR: missing ${ARCHIVE_PATH}"; exit 2; }
ACTUAL_SHA256="$(sha256sum "$ARCHIVE_PATH" | awk '{print $1}')"
[[ "$ACTUAL_SHA256" == "$EXPECTED_SHA256" ]] || {
  echo "ERROR: source archive SHA mismatch: ${ACTUAL_SHA256}"; exit 3;
}
mark "OPENDECK_V130_SOURCE_SHA256=PASS:${ACTUAL_SHA256}"

rm -rf "$WORK_ROOT"
mkdir -p "$TMP_ROOT" "${WORK_ROOT}/source"
export TMPDIR="$TMP_ROOT" TMP="$TMP_ROOT" TEMP="$TMP_ROOT"

FREE_KIB="$(df -Pk "$DOWNLOADS" | awk 'NR==2 {print $4}')"
FREE_INODES="$(df -Pi "$DOWNLOADS" | awk 'NR==2 {print $4}')"
mark "OPENDECK_V130_STORAGE_WORK_ROOT=${WORK_ROOT}"
mark "OPENDECK_V130_STORAGE_FREE_KIB=${FREE_KIB}"
mark "OPENDECK_V130_STORAGE_FREE_INODES=${FREE_INODES}"
(( FREE_KIB >= 4194304 )) || { echo "ERROR: need at least 4 GiB free in Downloads filesystem"; exit 4; }
if (( FREE_INODES > 0 )); then
  (( FREE_INODES >= 100000 )) || { echo "ERROR: fewer than 100000 free inodes"; exit 5; }
  mark "OPENDECK_V130_STORAGE_INODE_ACCOUNTING=ENFORCED"
else
  mark "OPENDECK_V130_STORAGE_INODE_ACCOUNTING=UNBOUNDED_OR_UNSUPPORTED"
fi
mark "OPENDECK_V130_STORAGE_PREFLIGHT=PASS"

tar -xJf "$ARCHIVE_PATH" -C "${WORK_ROOT}/source"
ROOT="${WORK_ROOT}/source/OpenDeck-Linux-v1.3.0-WINDOWS-SHELL-CHECKPOINT"
[[ -f "$ROOT/Cargo.toml" ]] || { echo "ERROR: extracted Cargo workspace missing"; exit 6; }
[[ -f "$ROOT/apps/opendeck-studio/package.json" ]] || { echo "ERROR: extracted Studio source missing"; exit 7; }
cd "$ROOT"
mark "OPENDECK_V130_EXTRACT=PASS:${ROOT}"

python3 scripts/verify/check-v130-phase1-4.py
python3 scripts/verify/check-hid-ownership.py
python3 scripts/verify/check-streamdeck-751-parity.py
python3 scripts/verify/clippy-closure.py --self-test
node --experimental-strip-types apps/opendeck-studio/src/parity/windows751Geometry.node.test.ts
node --experimental-strip-types apps/opendeck-studio/src/state/dragDrop.node.test.ts
python3 tests/v130/test_windows_shell_source.py
mark "OPENDECK_V130_SOURCE_AND_GEOMETRY_GATES=PASS"

set +e
PARITY_PREFLIGHT="$(python3 scripts/verify/check-v130-parity-matrix.py --preflight 2>&1)"
PARITY_RC=$?
set -e
printf '%s\n' "$PARITY_PREFLIGHT"
[[ $PARITY_RC -eq 2 ]] || { echo "ERROR: parity preflight returned unexpected rc=${PARITY_RC}"; exit 8; }
grep -Fq 'OPENDECK_V1_3_0_PARITY_EVIDENCE_PRESENT=8/12' <<<"$PARITY_PREFLIGHT" || { echo 'ERROR: unexpected parity evidence count'; exit 9; }
for marker in \
  'builtin_751_actions:apps/opendeck-studio/src/actions/catalog.windows751.test.ts' \
  'migration:tests/fixtures/v1.2.4-profile-store.json' \
  'screenshot_manifest:apps/opendeck-studio/src/parity/windows751Screenshots.test.ts' \
  'hardware_acceptance:tests/hardware/v1.3.0-plus-acceptance.sh'; do
  grep -Fq "$marker" <<<"$PARITY_PREFLIGHT" || { echo "ERROR: expected checkpoint gap missing: $marker"; exit 10; }
done
mark "OPENDECK_V130_FULL_PARITY_STATUS=INTENTIONALLY_INCOMPLETE:8_OF_12_EVIDENCE_CLASSES"

mark "OPENDECK_V130_FRONTEND_DEPENDENCIES=START"
npm --prefix apps/opendeck-studio ci --no-audit --no-fund
mark "OPENDECK_V130_FRONTEND_DEPENDENCIES=PASS"

npm --prefix apps/opendeck-studio test -- --run
mark "OPENDECK_V130_FRONTEND_TESTS=PASS"
npm --prefix apps/opendeck-studio run lint
mark "OPENDECK_V130_FRONTEND_LINT=PASS"
npm --prefix apps/opendeck-studio run build
mark "OPENDECK_V130_FRONTEND_BUILD=PASS"

cargo fmt --all
cargo fmt --all -- --check
mark "OPENDECK_V130_CARGO_FMT_PRECLOSURE=PASS"
cargo test --workspace --all-targets --all-features
mark "OPENDECK_V130_CARGO_TEST_PRECLOSURE=PASS"

python3 scripts/verify/clippy-closure.py "$ROOT" --log "$CLIPPY_LOG" --verify "$VERIFY"
mark "OPENDECK_V130_CLIPPY_CLOSURE=PASS"

# Requalify all behavior after any mechanical/scoped Clippy hygiene edits.
python3 scripts/verify/check-v130-phase1-4.py
python3 scripts/verify/check-hid-ownership.py
python3 scripts/verify/check-streamdeck-751-parity.py
node --experimental-strip-types apps/opendeck-studio/src/parity/windows751Geometry.node.test.ts
node --experimental-strip-types apps/opendeck-studio/src/state/dragDrop.node.test.ts
python3 tests/v130/test_windows_shell_source.py
npm --prefix apps/opendeck-studio test -- --run
npm --prefix apps/opendeck-studio run lint
npm --prefix apps/opendeck-studio run build
cargo fmt --all -- --check
cargo test --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features --keep-going -- -D warnings
mark "OPENDECK_V130_POST_CLOSURE_REQUALIFICATION=PASS"
mark "OPENDECK_V130_CARGO_CLIPPY_D_WARNINGS=PASS"

cargo build --workspace --all-features --release
mark "OPENDECK_V130_CARGO_RELEASE_BUILD=PASS"
npm --prefix apps/opendeck-studio run tauri -- build --no-bundle
[[ -x target/release/opendeck-studio ]] || { echo 'ERROR: preview Studio binary missing'; exit 11; }
mark "OPENDECK_V130_TAURI_NO_BUNDLE_BUILD=PASS"

# Preserve the exact host-clean staged source for the next parity phase.
rm -f "$CLEAN_ARCHIVE" "$CLEAN_SHA"
tar --exclude='./target' --exclude='./apps/opendeck-studio/node_modules' --exclude='./apps/opendeck-studio/dist' \
    -C "$ROOT" -cJf "$CLEAN_ARCHIVE" .
CLEAN_HASH="$(sha256sum "$CLEAN_ARCHIVE" | awk '{print $1}')"
printf '%s  %s\n' "$CLEAN_HASH" "$(basename "$CLEAN_ARCHIVE")" > "$CLEAN_SHA"
mark "OPENDECK_V130_HOST_CLEAN_SOURCE_SHA256=${CLEAN_HASH}"
mark "OPENDECK_V130_HOST_CLEAN_SOURCE_ARCHIVE=${CLEAN_ARCHIVE}"

DAEMON_STATE="unknown"
if command -v systemctl >/dev/null 2>&1; then
  DAEMON_STATE="$(systemctl --user is-active opendeck-daemon.service 2>/dev/null || true)"
  [[ -n "$DAEMON_STATE" ]] || DAEMON_STATE="inactive"
fi
mark "OPENDECK_V130_EXISTING_DAEMON_STATE=${DAEMON_STATE}"

# Launch only the staged Studio UI. Do not install/replace the canonical binaries or daemon.
nohup "$ROOT/target/release/opendeck-studio" > "$PREVIEW_LOG" 2>&1 &
PREVIEW_PID=$!
sleep 1
if kill -0 "$PREVIEW_PID" 2>/dev/null; then
  mark "OPENDECK_V130_WINDOWS_SHELL_PREVIEW=PASS:PID=${PREVIEW_PID}"
else
  mark "OPENDECK_V130_WINDOWS_SHELL_PREVIEW=STARTED_PROCESS_EXITED_EARLY:SEE=${PREVIEW_LOG}"
fi

mark "OPENDECK_V130_CANONICAL_INSTALL=NOT_PERFORMED"
mark "OPENDECK_V130_CANONICAL_RELEASE_GATE=DEFERRED_UNTIL_12_OF_12_PARITY_EVIDENCE"
mark "OPENDECK_V1_3_0_WINDOWS_SHELL_CHECKPOINT=PASS"
mark "OPENDECK_V1_3_0_TERMINAL_SESSION=PRESERVED"
SUCCESS=1
trap - EXIT
exit 0

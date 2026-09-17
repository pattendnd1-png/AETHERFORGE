#!/usr/bin/env bash
set -uo pipefail
DL="$HOME/Downloads"
TASK15_ANCHOR="dde8637a514309f5bcfe9f75dbb3ff97fe429cee"
COMMIT_MSG='test: gate canonical AetherFiles 0.1.0 release'
TMP="$DL/.AetherFiles-v0.1.0-TASK16-tmp"
PATCH="$TMP/AetherFiles-v0.1.0-TASK16-AETHERAI-GUARD-RECOVERY-PATCH.zip"
PATCH_ROOT="$TMP/patch"
LOGDIR="$DL/AetherFiles-v0.1.0-TASK16-logs"
VERIFY_EXTRACT="$DL/.AetherFiles-v0.1.0-final-verify"
CURRENT_LOG=""
ROOT=""
START_HEAD=""
MUTATED=0
COMMITTED=0
mkdir -p "$DL" "$TMP" "$LOGDIR"
say(){ printf '%s\n' "$1"; }
resolve_root(){
  local candidate
  for candidate in "$DL/AetherFiles-v0.1.0" "$DL/ForgeClean/Projects/AetherFiles/Active"; do
    if [[ -f "$candidate/Cargo.toml" && -d "$candidate/.git" ]]; then
      ROOT="$(cd "$candidate" && pwd -P)"
      return 0
    fi
  done
  return 1
}
print_tail(){
  if [[ -n "$CURRENT_LOG" && -f "$CURRENT_LOG" ]]; then
    printf '\n=== FIRST FAILURE DIAGNOSTIC (tail) ===\n'
    tail -n 240 "$CURRENT_LOG" || true
  fi
}
rollback(){
  if [[ "$MUTATED" -eq 1 && "$COMMITTED" -eq 0 && -n "$START_HEAD" && -d "$ROOT/.git" ]]; then
    cd "$ROOT" >/dev/null 2>&1 || true
    git reset --hard "$START_HEAD" >/dev/null 2>&1 || true
    git clean -fd -- scripts/build-release.sh scripts/verify-release.sh tests/test_final_release_contract.sh >/dev/null 2>&1 || true
  fi
}
fail(){
  local reason="$1"
  rollback
  printf '\n=== AETHERFILES TASK 16 RESULT ===\n'
  say "AETHERFILES_VERSION=0.1.0"
  say "AETHERFILES_TASK=TASK16_FINAL_RELEASE_AETHERAI_GUARD_RECOVERY"
  say "AETHERFILES_V0_1_0_TASK16=FAIL:$reason"
  say "AETHERFILES_HEAD_BEFORE=$START_HEAD"
  print_tail
  say "AETHERFILES_LOG_DIR=$LOGDIR"
  say "AETHERFILES_HANDOFF=PASTE_TERMINAL_RESULT"
  exit 1
}
run_gate(){
  local name="$1"; shift
  CURRENT_LOG="$LOGDIR/${name}.log"
  local rc=0
  "$@" >"$CURRENT_LOG" 2>&1 || rc=$?
  [[ $rc -eq 0 ]] || fail "$name"
  say "AETHERFILES_TASK16_${name}=PASS"
}
say "AETHERFILES_VERSION=0.1.0"
say "AETHERFILES_TASK=TASK16_FINAL_RELEASE_AETHERAI_GUARD_RECOVERY"
say "AETHERFILES_TASK16_SYSTEM_AETHERAI=EXTERNAL_SHARED_SERVICE"
say "AETHERFILES_TASK16_AETHERAI_NETWORK=DISABLED"
say "AETHERFILES_TASK16_AETHERAI_OWNED_RUNTIME=ABSENT"
say "AETHERFILES_TASK16_DEFAULT_FILE_MANAGER_TAKEOVER=DISABLED"
say "AETHERFILES_ARTIFACT_POLICY=ONE_CANONICAL_SET_IN_DOWNLOADS"
resolve_root || fail "PROJECT_ROOT_NOT_FOUND"
say "AETHERFILES_ROOT=$ROOT"
for cmd in cargo git python3 unzip sha256sum tar zstd; do
  command -v "$cmd" >/dev/null 2>&1 || fail "${cmd^^}_NOT_FOUND"
done
python3 - "$PATCH" <<'PY2'
from pathlib import Path
import base64, sys
payload = r"""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
ofEgotuIDlGAvuOQRugFjyY4vKGMSYs+0BY1kN97a/9AhJz0784+YofzdmY8A0cBk/Hc+Ti5KnRg
8Ok1cu8PCdz7Gqpa307XMy7KWbXcB7flkNE47GDX3Hb3j9LN44t0pFKu7W1JfAW164ZLahfq7Pro
8ekbGQ+Jq9dVO6uQ5pLNTnvY87f/TogHGBFMO6MgClEn2jXzl2hafwFQSwECFAMUAAAACAB1vzBd
raNkUzsBAAB4AgAADAAAAAAAAAAAAAAApIEAAAAATUFOSUZFU1QudHh0UEsBAhQDFAAAAAgAdb8w
XaQ9757ZAQAAggMAAB4AAAAAAAAAAAAAAO2BZQEAAGdyZWVuL3NjcmlwdHMvYnVpbGQtcmVsZWFz
ZS5zaFBLAQIUAxQAAAAIAHW/MF2WwBhKcwYAAN8RAAAfAAAAAAAAAAAAAADtgXoDAABncmVlbi9z
Y3JpcHRzL3ZlcmlmeS1yZWxlYXNlLnNoUEsBAhQDFAAAAAgAdb8wXb7bZsRbAQAAMwIAADAAAAAA
AAAAAAAAAO2BKgoAAGdyZWVuL3Rlc3RzL3Rlc3RfYWV0aGVyYWlfaW5zdGFsbF9ndWFyZF9zY29w
ZS5zaFBLAQIUAxQAAAAIAHW/MF2BbDbXQQEAAHcCAAAkAAAAAAAAAAAAAADtgdMLAABncmVlbi90
ZXN0cy90ZXN0X2Rlc2t0b3BfY29udHJhY3Quc2hQSwECFAMUAAAACAB1vzBdt1KfmksCAAAbBgAA
KgAAAAAAAAAAAAAA7YFWDQAAZ3JlZW4vdGVzdHMvdGVzdF9maW5hbF9yZWxlYXNlX2NvbnRyYWN0
LnNoUEsBAhQDFAAAAAgAdb8wXePSDsQ8AgAA2wUAACgAAAAAAAAAAAAAAO2B6Q8AAHJlZC90ZXN0
cy90ZXN0X2ZpbmFsX3JlbGVhc2VfY29udHJhY3Quc2hQSwUGAAAAAAcABwAxAgAAaxIAAAAA"""
out = Path(sys.argv[1])
out.parent.mkdir(parents=True, exist_ok=True)
out.write_bytes(base64.b64decode(payload))
PY2
EXPECTED="11857cc184c9a90a57a4f4a9cbe3f6b8d30d48bb1dd1f40df52cc2410f459cad"
ACTUAL="$(sha256sum "$PATCH" | awk '{print $1}')"
[[ "$ACTUAL" == "$EXPECTED" ]] || fail "PATCH_SHA256_MISMATCH"
say "AETHERFILES_TASK16_EMBEDDED_PATCH_SHA256=PASS"
rm -rf "$PATCH_ROOT" && mkdir -p "$PATCH_ROOT"
unzip -q "$PATCH" -d "$PATCH_ROOT" || fail "PATCH_UNZIP"
say "AETHERFILES_TASK16_PATCH_UNZIP=PASS"
cd "$ROOT" || fail "PROJECT_ROOT_CD"
START_HEAD="$(git rev-parse HEAD 2>/dev/null)" || fail "GIT_HEAD"
[[ "$START_HEAD" == "$TASK15_ANCHOR" ]] || fail "WRONG_TASK15_ANCHOR:$START_HEAD"
say "AETHERFILES_TASK16_TASK15_ANCHOR=PASS"
[[ -z "$(git status --porcelain=v1 -uall)" ]] || {
  CURRENT_LOG="$LOGDIR/DIRTY_WORKTREE.log"
  git status --short --untracked-files=all >"$CURRENT_LOG" 2>&1 || true
  fail "DIRTY_WORKTREE_BEFORE_TASK16"
}
say "AETHERFILES_TASK16_PRE_RUN_CLEAN=PASS"
mkdir -p tests
cp "$PATCH_ROOT/red/tests/test_final_release_contract.sh" tests/test_final_release_contract.sh
chmod +x tests/test_final_release_contract.sh
MUTATED=1
CURRENT_LOG="$LOGDIR/RED_FINAL_RELEASE_CONTRACT.log"
set +e
bash tests/test_final_release_contract.sh >"$CURRENT_LOG" 2>&1
RED_RC=$?
set -e
[[ $RED_RC -ne 0 ]] || fail "RED_TEST_UNEXPECTEDLY_PASSED"
say "AETHERFILES_TASK16_RED_FINAL_RELEASE_CONTRACT=PASS"
mkdir -p scripts tests
cp "$PATCH_ROOT/green/scripts/build-release.sh" scripts/build-release.sh
cp "$PATCH_ROOT/green/scripts/verify-release.sh" scripts/verify-release.sh
cp "$PATCH_ROOT/green/tests/test_final_release_contract.sh" tests/test_final_release_contract.sh
cp "$PATCH_ROOT/green/tests/test_desktop_contract.sh" tests/test_desktop_contract.sh
cp "$PATCH_ROOT/green/tests/test_aetherai_install_guard_scope.sh" tests/test_aetherai_install_guard_scope.sh
chmod +x scripts/build-release.sh scripts/verify-release.sh tests/test_final_release_contract.sh tests/test_desktop_contract.sh tests/test_aetherai_install_guard_scope.sh
grep -Fq 'section_heading(ui, "PLACES")' crates/aetherfiles-ui/src/sidebar.rs \
  || fail "TASK15_PLACES_SECTION_MISSING"
grep -Fq 'section_heading(ui, "PLACES")' tests/test_desktop_contract.sh \
  || fail "DESKTOP_CONTRACT_RECOVERY_MISSING"
say "AETHERFILES_TASK16_DESKTOP_CONTRACT_RECOVERY=PASS"
bash tests/test_aetherai_install_guard_scope.sh >/dev/null 2>&1 || fail "AETHERAI_INSTALL_GUARD_SCOPE_RECOVERY"
say "AETHERFILES_TASK16_AETHERAI_GUARD_RECOVERY=PASS"
python3 - <<'PYDOC'
from pathlib import Path
readme = Path('README.md')
marker = '<!-- AETHERFILES_V0_1_0_CANONICAL_VERIFY -->'
block = """

<!-- AETHERFILES_V0_1_0_CANONICAL_VERIFY -->
## AetherFiles v0.1.0 canonical verification

The canonical source release is `AetherFiles-v0.1.0-SOURCE.tar.zst`. Verify it from an isolated extracted tree:

```bash
cd "$HOME/Downloads" && rm -rf .AetherFiles-v0.1.0-verify && mkdir .AetherFiles-v0.1.0-verify && tar --zstd -xf AetherFiles-v0.1.0-SOURCE.tar.zst -C .AetherFiles-v0.1.0-verify && cd .AetherFiles-v0.1.0-verify/AetherFiles-v0.1.0 && bash scripts/verify-release.sh
```

AetherAI remains the external shared system service; AetherFiles does not install or own an AI runtime. AetherFiles is not the default `inode/directory` handler in v0.1.0.
"""
text = readme.read_text() if readme.exists() else '# AetherFiles\n'
if marker not in text:
    readme.write_text(text.rstrip() + block + '\n')
changelog = Path('CHANGELOG.md')
cm = '<!-- AETHERFILES_V0_1_0_RELEASE_GATE -->'
cb = """

<!-- AETHERFILES_V0_1_0_RELEASE_GATE -->
### v0.1.0 release qualification

- Added clean-package verification, strict workspace Clippy/tests/release-build gates, integration contracts, and canonical source/checksum handoff.
- Preserved shared system-level local-only AetherAI authority; no AetherFiles-owned model/runtime/service install.
- Preserved Dolphin/default file-manager rollback safety; v0.1.0 does not take over `inode/directory`.
"""
ct = changelog.read_text() if changelog.exists() else '# Changelog\n'
if cm not in ct:
    changelog.write_text(ct.rstrip() + cb + '\n')
PYDOC
say "AETHERFILES_TASK16_GREEN_PATCH=APPLIED"
run_gate "FINAL_RELEASE_CONTRACT" bash tests/test_final_release_contract.sh
run_gate "FMT" cargo fmt --all --check
run_gate "CHECK" cargo check --workspace --all-targets
run_gate "CLIPPY" cargo clippy --workspace --all-targets -- -D warnings
run_gate "TESTS" cargo test --workspace
run_gate "RELEASE_BUILD" cargo build --workspace --release
for contract in tests/test_desktop_contract.sh tests/test_no_dolphin_runtime.sh tests/test_desktop_integration.sh tests/test_aether_terminal_ui_contract.sh tests/test_aetherai_capability_boundary.sh tests/test_aetherai_install_guard_scope.sh tests/test_forgeclean_service_contract.sh tests/test_task15_dragonglass_ux.sh; do
  [[ -f "$contract" ]] || fail "MISSING_REQUIRED_CONTRACT:${contract//\//_}"
  run_gate "CONTRACT_$(basename "$contract" .sh | tr '[:lower:]-' '[:upper:]_')" bash "$contract"
done
! grep -RqsE 'xdg-mime[[:space:]]+default|gio[[:space:]]+mime[[:space:]]+inode/directory' scripts || fail "DEFAULT_FILE_MANAGER_TAKEOVER_FOUND"
if grep -Eqs 'systemctl( --user)? .*aetherai|libexec/aetherfiles/aetherai' scripts/install-local.sh; then
  fail "AETHERFILES_OWNED_AETHERAI_INSTALL_FOUND"
fi
if grep -RqsE 'systemctl( --user)? .*aetherai|libexec/aetherfiles/aetherai' resources; then
  fail "AETHERFILES_OWNED_AETHERAI_INSTALL_FOUND"
fi
grep -Rqs '/usr/bin/aether-terminal' crates/aetherfiles-ui/src || fail "AETHER_TERMINAL_AUTHORITY_MISSING"
say "AETHERFILES_TASK16_ARCHITECTURE_INVARIANTS=PASS"
git add scripts/build-release.sh scripts/verify-release.sh tests/test_final_release_contract.sh tests/test_desktop_contract.sh tests/test_aetherai_install_guard_scope.sh README.md CHANGELOG.md
git diff --cached --check || fail "GIT_DIFF_CHECK"
git diff --cached --quiet && fail "NOTHING_TO_COMMIT"
git commit -m "$COMMIT_MSG" >"$LOGDIR/GIT_COMMIT.log" 2>&1 || { CURRENT_LOG="$LOGDIR/GIT_COMMIT.log"; fail "GIT_COMMIT"; }
COMMITTED=1
TASK16_HEAD="$(git rev-parse HEAD)"
say "AETHERFILES_TASK16_GIT_COMMIT=PASS"
say "AETHERFILES_TASK16_NEW_HEAD=$TASK16_HEAD"
[[ -z "$(git status --porcelain=v1 -uall)" ]] || { CURRENT_LOG="$LOGDIR/POST_COMMIT_DIRTY.log"; git status --short --untracked-files=all >"$CURRENT_LOG" 2>&1 || true; fail "DIRTY_WORKTREE_AFTER_TASK16_COMMIT"; }
say "AETHERFILES_TASK16_POST_COMMIT_CLEAN=PASS"
run_gate "BUILD_CANONICAL_SOURCE" bash scripts/build-release.sh
SOURCE="$DL/AetherFiles-v0.1.0-SOURCE.tar.zst"
VERIFY="$DL/AetherFiles-v0.1.0-VERIFY.txt"
SUMS="$DL/AetherFiles-v0.1.0-SHA256SUMS.txt"
[[ -s "$SOURCE" ]] || fail "SOURCE_ARCHIVE_MISSING"
[[ -s "$SUMS" ]] || fail "INITIAL_SUMS_MISSING"
CURRENT_LOG="$LOGDIR/ARCHIVE_LAYOUT.log"
tar --zstd -tf "$SOURCE" >"$CURRENT_LOG" 2>&1 || fail "ARCHIVE_LIST"
grep -Fxq 'AetherFiles-v0.1.0/scripts/verify-release.sh' "$CURRENT_LOG" || fail "ARCHIVE_VERIFY_SCRIPT_MISSING"
grep -Fxq 'AetherFiles-v0.1.0/scripts/build-release.sh' "$CURRENT_LOG" || fail "ARCHIVE_BUILD_SCRIPT_MISSING"
grep -Fxq 'AetherFiles-v0.1.0/README.md' "$CURRENT_LOG" || fail "ARCHIVE_README_MISSING"
! grep -q '/target/' "$CURRENT_LOG" || fail "ARCHIVE_CONTAINS_TARGET"
! grep -q '/.git/' "$CURRENT_LOG" || fail "ARCHIVE_CONTAINS_GIT_METADATA"
say "AETHERFILES_TASK16_ARCHIVE_LAYOUT=PASS"
rm -rf "$VERIFY_EXTRACT" && mkdir -p "$VERIFY_EXTRACT"
tar --zstd -xf "$SOURCE" -C "$VERIFY_EXTRACT" || fail "CLEAN_PACKAGE_EXTRACT"
PACKAGE_ROOT="$VERIFY_EXTRACT/AetherFiles-v0.1.0"
[[ -f "$PACKAGE_ROOT/Cargo.toml" ]] || fail "CLEAN_PACKAGE_ROOT_MISSING"
run_gate "CLEAN_PACKAGE_VERIFY" bash "$PACKAGE_ROOT/scripts/verify-release.sh"
[[ -s "$VERIFY" ]] || fail "VERIFY_FILE_MISSING"
grep -Fxq 'AETHERFILES_V0_1_0_VERIFY=PASS' "$VERIFY" || fail "VERIFY_ENDPOINT_MISSING"
say "AETHERFILES_TASK16_CLEAN_PACKAGE_ENDPOINT=PASS"
(
  cd "$DL" || exit 1
  sha256sum AetherFiles-v0.1.0-SOURCE.tar.zst AetherFiles-v0.1.0-VERIFY.txt > AetherFiles-v0.1.0-SHA256SUMS.txt
) || fail "FINAL_SHA256_MANIFEST"
run_gate "CANONICAL_SHA256" bash -c 'cd "$1" && sha256sum -c AetherFiles-v0.1.0-SHA256SUMS.txt' _ "$DL"
SOURCE_SHA="$(sha256sum "$SOURCE" | awk '{print $1}')"
VERIFY_SHA="$(sha256sum "$VERIFY" | awk '{print $1}')"
SUMS_SHA="$(sha256sum "$SUMS" | awk '{print $1}')"
printf '\n=== AETHERFILES TASK 16 RESULT ===\n'
say "AETHERFILES_VERSION=0.1.0"
say "AETHERFILES_TASK=TASK16_FINAL_RELEASE_AETHERAI_GUARD_RECOVERY"
say "AETHERFILES_TASK16_TASK15_HEAD=$TASK15_ANCHOR"
say "AETHERFILES_TASK16_NEW_HEAD=$TASK16_HEAD"
say "AETHERFILES_CANONICAL_SOURCE=$SOURCE"
say "AETHERFILES_CANONICAL_SOURCE_SHA256=$SOURCE_SHA"
say "AETHERFILES_CANONICAL_VERIFY=$VERIFY"
say "AETHERFILES_CANONICAL_VERIFY_SHA256=$VERIFY_SHA"
say "AETHERFILES_CANONICAL_SUMS=$SUMS"
say "AETHERFILES_CANONICAL_SUMS_SHA256=$SUMS_SHA"
say "AETHERFILES_SYSTEM_AETHERAI=EXTERNAL_SHARED_SERVICE"
say "AETHERFILES_AETHERAI_NETWORK=DISABLED"
say "AETHERFILES_AETHERAI_OWNED_RUNTIME=ABSENT"
say "AETHERFILES_DEFAULT_FILE_MANAGER_TAKEOVER=DISABLED"
say "AETHERFILES_V0_1_0_VERIFY=PASS"
say "AETHERFILES_V0_1_0_TASK16=PASS"
say "AETHERFILES_LOG_DIR=$LOGDIR"
say "AETHERFILES_HANDOFF=PASTE_TERMINAL_RESULT"
rm -rf "$TMP" "$VERIFY_EXTRACT" >/dev/null 2>&1 || true

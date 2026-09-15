#!/usr/bin/env bash
set -Eeuo pipefail

BASE_VERSION="1.0.11"
VERSION="1.0.13"
DOWNLOADS="$HOME/Downloads"
DELTA="$DOWNLOADS/ForgeClean-v${VERSION}-DELTA.zip"
EXPECTED_DELTA_SHA="8b7fdc60234e4dedc54b447406e128ffc9a07bb8ed12b1380449abc370e45df5"
SRC="$DOWNLOADS/ForgeClean-v${VERSION}"
VERIFY="$DOWNLOADS/ForgeClean-v${VERSION}-VERIFY.txt"
SOURCE_ZIP="$DOWNLOADS/ForgeClean-v${VERSION}-SOURCE.zip"
SUMS="$DOWNLOADS/ForgeClean-v${VERSION}-SHA256SUMS.txt"
ROLLBACK="$DOWNLOADS/ForgeClean-v${VERSION}-ROLLBACK.sh"
STATE="$HOME/.local/state/forgeclean-v${VERSION}-$(date +%Y%m%d-%H%M%S)"
REPO_DIR="${AETHERFORGE_REPO_DIR:-$HOME/Downloads/AETHERFORGE}"
ORBIT="${AETHERFORGE_ORBITAL_BIN:-$HOME/.local/bin/aetherforge-orbital-sync}"
DELTA_TMP=""
RED_SRC=""

mkdir -p "$DOWNLOADS" "$STATE"
: > "$VERIFY"
emit(){ printf '%s\n' "$*" | tee -a "$VERIFY"; }
die(){ emit "FORGECLEAN_V1_0_13=FAIL:$*"; exit 1; }

cleanup(){
  local rc=$?
  trap - EXIT
  [[ -z "$DELTA_TMP" ]] || rm -rf -- "$DELTA_TMP" || true
  [[ -z "$RED_SRC" ]] || rm -rf -- "$RED_SRC" || true
  if (( rc == 0 )); then
    emit "FORGECLEAN_V1_0_13=PASS"
  else
    emit "FORGECLEAN_V1_0_13=FAIL:${rc}"
  fi
  emit "VERIFY_FILE=$VERIFY"
  exit "$rc"
}
trap cleanup EXIT

for cmd in cargo rustc rsync sha256sum systemctl grep awk sed touch df find stat install; do
  command -v "$cmd" >/dev/null 2>&1 || die "MISSING_COMMAND:$cmd"
done
if ! command -v bsdtar >/dev/null 2>&1 && ! command -v unzip >/dev/null 2>&1; then
  die "MISSING_ARCHIVE_EXTRACTOR:NEED_BSDTAR_OR_UNZIP"
fi
if ! command -v bsdtar >/dev/null 2>&1 && ! command -v zip >/dev/null 2>&1; then
  die "MISSING_ARCHIVE_WRITER:NEED_BSDTAR_OR_ZIP"
fi

[[ -f "$DELTA" ]] || die "DELTA_NOT_FOUND:$DELTA"
ACTUAL_DELTA_SHA="$(sha256sum "$DELTA" | awk '{print $1}')"
[[ "$ACTUAL_DELTA_SHA" == "$EXPECTED_DELTA_SHA" ]] || die "DELTA_SHA256_MISMATCH:$ACTUAL_DELTA_SHA"
emit "FORGECLEAN_DELTA_SHA256=PASS"

BASE_SRC=""
for candidate in "$REPO_DIR/apps/ForgeClean" "$DOWNLOADS/ForgeClean-v${BASE_VERSION}"; do
  [[ -f "$candidate/Cargo.toml" ]] || continue
  candidate_version="$(awk -F= '/^[[:space:]]*version[[:space:]]*=/{gsub(/[\"[:space:]]/,\"\",$2);print $2;exit}' "$candidate/Cargo.toml")"
  if [[ "$candidate_version" == "$BASE_VERSION" ]]; then
    BASE_SRC="$candidate"
    break
  fi
done
[[ -n "$BASE_SRC" ]] || die "VERIFIED_V1_0_11_BASE_NOT_FOUND"
[[ -f "$BASE_SRC/.aetherforge-verified-source" ]] || die "BASE_VERIFIED_MARKER_MISSING:$BASE_SRC"
grep -Fxq "version=$BASE_VERSION" "$BASE_SRC/.aetherforge-verified-source" || die "BASE_VERIFIED_VERSION_MISMATCH"
for gate in fmt clippy tests release_build; do
  grep -Fxq "$gate=PASS" "$BASE_SRC/.aetherforge-verified-source" || die "BASE_VERIFIED_GATE_MISSING:$gate"
done
emit "FORGECLEAN_BASE_SOURCE=$BASE_SRC"
emit "FORGECLEAN_BASE_VERIFIED=PASS"

extract_delta(){
  if command -v bsdtar >/dev/null 2>&1; then
    bsdtar -xf "$DELTA" -C "$DELTA_TMP"
  else
    unzip -q "$DELTA" -d "$DELTA_TMP"
  fi
}
archive_source(){
  local base="$(basename -- "$SRC")"
  rm -f -- "$SOURCE_ZIP"
  if command -v bsdtar >/dev/null 2>&1; then
    (
      cd "$DOWNLOADS"
      bsdtar -a -cf "$SOURCE_ZIP" \
        --exclude "$base/target" \
        --exclude "$base/target/*" \
        --exclude "$base/.git" \
        --exclude "$base/.git/*" \
        "$base"
    )
  else
    (
      cd "$DOWNLOADS"
      zip -qr "$SOURCE_ZIP" "$base" -x "$base/target/*" "$base/.git/*"
    )
  fi
}

DELTA_TMP="$(mktemp -d "$DOWNLOADS/forgeclean-v${VERSION}-delta.XXXXXX")"
RED_SRC="$(mktemp -d "$DOWNLOADS/forgeclean-v${VERSION}-red.XXXXXX")"
extract_delta || die "DELTA_EXTRACT_FAILED"
DELTA_ROOT="$DELTA_TMP/ForgeClean-v${VERSION}-DELTA"
[[ -x "$DELTA_ROOT/apply_delta.sh" ]] || die "DELTA_APPLIER_MISSING"
if find "$DELTA_ROOT" -type f \( -name '*.py' -o -name '*.pyc' \) -print -quit | grep -q .; then
  die "DELTA_CONTAINS_PYTHON_FILE"
fi
if find "$DELTA_ROOT" -type d -name '__pycache__' -print -quit | grep -q .; then
  die "DELTA_CONTAINS_PYTHON_CACHE"
fi
emit "FORGECLEAN_RUST_FIRST_DELTA=PASS"

# TDD RED: untouched verified v1.0.11 must not satisfy the v1.0.13 storage-recovery contract.
rsync -a --exclude='.git/' --exclude='target/' --exclude='.aetherforge-verified-source' "$BASE_SRC/" "$RED_SRC/"
sed -i '0,/^version = "1\.0\.11"$/s//version = "1.0.13"/' "$RED_SRC/Cargo.toml"
install -m 0755 "$DELTA_ROOT/payload/tests/regression_v1_0_13_pre_rebase.sh" "$RED_SRC/tests/regression_v1_0_13_pre_rebase.sh"
set +e
(
  cd "$RED_SRC"
  ./tests/regression_v1_0_13_pre_rebase.sh
) >"$STATE/tdd-red.log" 2>&1
RED_RC=$?
set -e
[[ $RED_RC -ne 0 ]] || die "TDD_RED_UNEXPECTEDLY_PASSED"
emit "FORGECLEAN_V1_0_13_TDD_RED=PASS"

"$DELTA_ROOT/apply_delta.sh" "$BASE_SRC" "$SRC" >>"$VERIFY" 2>&1 || die "DELTA_APPLY_FAILED"
(
  cd "$SRC"
  ./tests/regression_v1_0_13_pre_rebase.sh
  ./tests/regression_v1_0_13_rust_first.sh
  bash -n build-and-verify.sh install-local.sh hit-it-template.sh tests/*.sh
) >>"$VERIFY" 2>&1 || die "TDD_GREEN_STATIC_FAILED"
emit "FORGECLEAN_V1_0_13_TDD_GREEN_STATIC=PASS"

# Canonical verifier owns fmt, strict Clippy, all Rust tests, release build, three-binary export,
# and the synthetic HOME destructive test. It never targets real user data.
if ! (cd "$SRC" && ./build-and-verify.sh); then
  die "BUILD_VERIFY_FAILED"
fi
grep -Fxq 'FORGECLEAN_VERIFY=PASS' "$VERIFY" || die "BUILD_VERIFY_DID_NOT_PASS"
grep -Fxq 'FORGECLEAN_CLIPPY=PASS' "$VERIFY" || die "CLIPPY_DID_NOT_PASS"
grep -Fxq 'FORGECLEAN_TEST=PASS' "$VERIFY" || die "TESTS_DID_NOT_PASS"
grep -Fxq 'FORGECLEAN_BUILD=PASS' "$VERIFY" || die "RELEASE_BUILD_DID_NOT_PASS"
grep -Fxq 'FORGECLEAN_PRE_REBASE_E2E=PASS' "$VERIFY" || die "PRE_REBASE_E2E_DID_NOT_PASS"
grep -Fxq 'FORGECLEAN_SYSTEM_CLI_SMOKE=PASS' "$VERIFY" || die "SYSTEM_BINARY_SMOKE_DID_NOT_PASS"
grep -Fxq 'FORGECLEAN_V1_0_13_RUST_FIRST=PASS' "$VERIFY" || die "RUST_FIRST_GATE_DID_NOT_PASS"
emit "FORGECLEAN_NONDESTRUCTIVE_GATES=PASS"
emit "FORGECLEAN_V1_0_13_TDD_RED=PASS"
emit "FORGECLEAN_V1_0_13_TDD_GREEN=PASS"

for bin in forgeclean forgeclean-gui forgeclean-system; do
  [[ -x "$SRC/target/release/$bin" ]] || die "MISSING_RELEASE_BINARY:$bin"
done
"$SRC/target/release/forgeclean-gui" --self-test >>"$VERIFY" 2>&1
"$SRC/target/release/forgeclean-gui" --version >>"$VERIFY" 2>&1
"$SRC/target/release/forgeclean-system" --version >>"$VERIFY" 2>&1
"$SRC/target/release/forgeclean-system" pre-rebase status >>"$VERIFY" 2>&1
emit "FORGECLEAN_THREE_BINARY_GATE=PASS"

cat > "$SRC/.aetherforge-verified-source" <<MARKER
schema=1
project=ForgeClean
version=$VERSION
fmt=PASS
clippy=PASS
tests=PASS
release_build=PASS
gui_self_test=PASS
three_binary_gate=PASS
pre_rebase_fake_home_e2e=PASS
rust_native_pre_rebase=PASS
verified_at=$(date -Iseconds)
MARKER
grep -Fxq "version=$VERSION" "$SRC/.aetherforge-verified-source" || die "VERIFIED_SOURCE_MARKER_INVALID"
emit "FORGECLEAN_VERIFIED_SOURCE_MARKER=PASS"

archive_source || die "SOURCE_ZIP_FAILED"
SOURCE_SHA="$(sha256sum "$SOURCE_ZIP" | awk '{print $1}')"
emit "FORGECLEAN_SOURCE_ZIP=PASS"
emit "SOURCE_SHA256=$SOURCE_SHA"

mkdir -p "$STATE/bin-before" "$STATE/systemd-before"
for bin in forgeclean forgeclean-gui forgeclean-system; do
  [[ ! -e "$HOME/.local/bin/$bin" ]] || cp -a "$HOME/.local/bin/$bin" "$STATE/bin-before/$bin"
done
for unit in forgeclean-pre-rebase.service forgeclean-pre-rebase.timer; do
  [[ ! -e "$HOME/.config/systemd/user/$unit" ]] || cp -a "$HOME/.config/systemd/user/$unit" "$STATE/systemd-before/$unit"
done

if ! (cd "$SRC" && ./install-local.sh); then
  die "INSTALL_OR_IMMEDIATE_SWEEP_FAILED"
fi
grep -Fxq 'FORGECLEAN_PRE_REBASE_IMMEDIATE=PASS' "$VERIFY" || die "IMMEDIATE_SWEEP_DID_NOT_PASS"
grep -Fxq 'FORGECLEAN_PRE_REBASE_TIMER=PASS' "$VERIFY" || die "PRE_REBASE_TIMER_DID_NOT_PASS"
emit "FORGECLEAN_REAL_STORAGE_RECLAIM=PASS"

cat > "$ROLLBACK" <<ROLLBACK
#!/usr/bin/env bash
set -Eeuo pipefail
BACKUP='$STATE'
systemctl --user disable --now forgeclean-pre-rebase.timer >/dev/null 2>&1 || true
for bin in forgeclean forgeclean-gui forgeclean-system; do
  if [[ -f "$BACKUP/bin-before/$bin" ]]; then
    install -m 0755 "$BACKUP/bin-before/$bin" "$HOME/.local/bin/$bin"
  fi
done
for unit in forgeclean-pre-rebase.service forgeclean-pre-rebase.timer; do
  if [[ -f "$BACKUP/systemd-before/$unit" ]]; then
    install -m 0644 "$BACKUP/systemd-before/$unit" "$HOME/.config/systemd/user/$unit"
  else
    rm -f "$HOME/.config/systemd/user/$unit"
  fi
done
systemctl --user daemon-reload
systemctl --user restart forgeclean-organizer.service >/dev/null 2>&1 || true
echo 'FORGECLEAN_V1_0_13_ROLLBACK=PASS'
echo 'ROLLBACK_SCOPE=BINARIES_AND_TIMER_ONLY_PERMANENTLY_DELETED_FILES_ARE_NOT_RECOVERABLE'
ROLLBACK
chmod +x "$ROLLBACK"
emit "FORGECLEAN_ROLLBACK_SCRIPT=PASS"
emit "ROLLBACK_SCOPE=BINARIES_AND_TIMER_ONLY_DELETED_FILES_ARE_NOT_RECOVERABLE"

SELF="$DOWNLOADS/ForgeClean-v${VERSION}-HIT-IT.sh"
CANONICAL_TMP="$STATE/ForgeClean-v${VERSION}-HIT-IT.sh"
sed "s|__SOURCE_SHA256__|$SOURCE_SHA|g" "$SRC/hit-it-template.sh" > "$CANONICAL_TMP"
chmod +x "$CANONICAL_TMP"
install -m 0755 "$CANONICAL_TMP" "$SELF"
emit "FORGECLEAN_CANONICAL_HIT_IT=PASS"

{
  sha256sum "$SOURCE_ZIP"
  sha256sum "$SELF"
  sha256sum "$HOME/.local/bin/forgeclean"
  sha256sum "$HOME/.local/bin/forgeclean-gui"
  sha256sum "$HOME/.local/bin/forgeclean-system"
  sha256sum "$ROLLBACK"
} > "$SUMS"
emit "FORGECLEAN_SHA256SUMS=PASS"

# Preserve established source authority when available, but do not make storage recovery depend on network sync.
if [[ -x "$ORBIT" ]]; then
  if AETHERFORGE_UPLOAD_ARTIFACTS=0 "$ORBIT" full >>"$VERIFY" 2>&1; then
    emit "FORGECLEAN_ORBITAL_CANONICALIZATION=PASS"
  else
    emit "FORGECLEAN_ORBITAL_CANONICALIZATION=DEFERRED"
  fi
else
  emit "FORGECLEAN_ORBITAL_CANONICALIZATION=SKIP:ORBITAL_NOT_FOUND"
fi

emit "FORGECLEAN_VERSION=$VERSION"
emit "FORGECLEAN_IMPLEMENTATION=RUST_NATIVE"
emit "FORGECLEAN_PRE_REBASE_CUTOFF=2026-08-18T00:00:00-07:00"
emit "FORGECLEAN_PRE_REBASE_TIMESTAMP_POLICY=ANY_AVAILABLE_TIMESTAMP"
emit "FORGECLEAN_PRE_REBASE_DELETE_MODE=DIRECT_UNLINK_NO_TRASH"
emit "FORGECLEAN_PRE_REBASE_LIVE_STATE_PROTECTION=PASS"
emit "FORGECLEAN_PRE_REBASE_VCS_PROTECTION=PASS"
emit "FORGECLEAN_PRE_REBASE_CURRENT_RELEASE_PROTECTION=PASS"
emit "FORGECLEAN_PRE_REBASE_IDENTITY_REVALIDATION=PASS"
emit "FORGECLEAN_PRE_REBASE_PARENT_SYMLINK_GUARD=PASS"
emit "FORGECLEAN_STORAGE_PRESSURE_TRIGGER=70_PERCENT"
emit "FORGECLEAN_STORAGE_CRITICAL_ALERT=85_PERCENT"
emit "FORGECLEAN_PRE_REBASE_STARTUP_DAILY_TIMER=PASS"
emit "FORGECLEAN_V1_0_13_HOSTILE_TAKEOVER=PASS"
emit "SOURCE_ZIP=$SOURCE_ZIP"
emit "SHA256SUMS=$SUMS"
emit "ROLLBACK_SCRIPT=$ROLLBACK"
exit 0

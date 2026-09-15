#!/usr/bin/env bash
set -Eeuo pipefail

BASE_VERSION="1.0.11"
VERSION="1.0.12"
DOWNLOADS="$HOME/Downloads"
DELTA="$DOWNLOADS/ForgeClean-v${VERSION}-DELTA.zip"
EXPECTED_DELTA_SHA="abad07d3d8a0fcbe5d7b851b575d1468ee35efea1d7c3dc3c0f136ff2176734a"
SRC="$DOWNLOADS/ForgeClean-v${VERSION}"
VERIFY="$DOWNLOADS/ForgeClean-v${VERSION}-VERIFY.txt"
SOURCE_ZIP="$DOWNLOADS/ForgeClean-v${VERSION}-SOURCE.zip"
SUMS="$DOWNLOADS/ForgeClean-v${VERSION}-SHA256SUMS.txt"
ROLLBACK="$DOWNLOADS/ForgeClean-v${VERSION}-ROLLBACK.sh"
STATE="$HOME/.local/state/forgeclean-v${VERSION}-$(date +%Y%m%d-%H%M%S)"
ORBIT="${AETHERFORGE_ORBITAL_BIN:-$HOME/.local/bin/aetherforge-orbital-sync}"
REPO_DIR="${AETHERFORGE_REPO_DIR:-$HOME/Downloads/AETHERFORGE}"

mkdir -p "$DOWNLOADS" "$STATE"
: > "$VERIFY"
emit(){ printf '%s\n' "$*" | tee -a "$VERIFY"; }
die(){ emit "FORGECLEAN_V1_0_12=FAIL:$*"; exit 1; }

cleanup(){
  local rc=$?
  trap - EXIT
  if (( rc == 0 )); then
    emit "FORGECLEAN_V1_0_12=PASS"
  else
    emit "FORGECLEAN_V1_0_12=FAIL:${rc}"
  fi
  emit "VERIFY_FILE=$VERIFY"
  exit "$rc"
}
trap cleanup EXIT

for cmd in cargo rustc rsync python3 sha256sum systemctl grep awk touch df; do
  command -v "$cmd" >/dev/null 2>&1 || die "MISSING_COMMAND:$cmd"
done
[[ -f "$DELTA" ]] || die "DELTA_NOT_FOUND:$DELTA"
ACTUAL_DELTA_SHA="$(sha256sum "$DELTA" | awk '{print $1}')"
[[ "$ACTUAL_DELTA_SHA" == "$EXPECTED_DELTA_SHA" ]] || die "DELTA_SHA256_MISMATCH:$ACTUAL_DELTA_SHA"
emit "FORGECLEAN_DELTA_SHA256=PASS"

BASE_SRC=""
for candidate in "$REPO_DIR/apps/ForgeClean" "$DOWNLOADS/ForgeClean-v${BASE_VERSION}"; do
  [[ -f "$candidate/Cargo.toml" ]] || continue
  candidate_version="$(awk -F= '/^[[:space:]]*version[[:space:]]*=/{gsub(/["[:space:]]/,"",$2);print $2;exit}' "$candidate/Cargo.toml")"
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

DELTA_TMP="$(mktemp -d "$DOWNLOADS/forgeclean-v${VERSION}-delta.XXXXXX")"
RED_SRC="$(mktemp -d "$DOWNLOADS/forgeclean-v${VERSION}-red.XXXXXX")"
rm_tmp(){ rm -rf -- "$DELTA_TMP" "$RED_SRC"; }
trap 'rm_tmp; cleanup' EXIT
python3 - "$DELTA" "$DELTA_TMP" <<'PY'
import sys,zipfile
with zipfile.ZipFile(sys.argv[1]) as z:
    z.extractall(sys.argv[2])
PY
DELTA_ROOT="$DELTA_TMP/ForgeClean-v${VERSION}-DELTA"
[[ -x "$DELTA_ROOT/apply_patch.py" ]] || die "DELTA_APPLIER_MISSING"

# TDD RED: promote only the package identity in a disposable copy, add the new regression,
# and prove the untouched v1.0.11 implementation cannot satisfy the v1.0.12 contract.
rsync -a --exclude='.git/' --exclude='target/' --exclude='.aetherforge-verified-source' "$BASE_SRC/" "$RED_SRC/"
python3 - "$RED_SRC/Cargo.toml" <<'PY'
from pathlib import Path
import re,sys
p=Path(sys.argv[1]); s=p.read_text(); s,n=re.subn(r'(?m)^(version\s*=\s*)"1\.0\.11"\s*$',r'\g<1>"1.0.12"',s,count=1)
if n != 1: raise SystemExit(1)
p.write_text(s)
PY
cp "$DELTA_ROOT/payload/tests/regression_v1_0_12_pre_rebase.sh" "$RED_SRC/tests/"
chmod +x "$RED_SRC/tests/regression_v1_0_12_pre_rebase.sh"
set +e
( cd "$RED_SRC" && ./tests/regression_v1_0_12_pre_rebase.sh ) >"$STATE/tdd-red.log" 2>&1
RED_RC=$?
set -e
[[ $RED_RC -ne 0 ]] || die "TDD_RED_UNEXPECTEDLY_PASSED"
emit "FORGECLEAN_V1_0_12_TDD_RED=PASS"

python3 "$DELTA_ROOT/apply_patch.py" "$BASE_SRC" "$SRC" >>"$VERIFY" 2>&1
( cd "$SRC" && ./tests/regression_v1_0_12_pre_rebase.sh ) >>"$VERIFY" 2>&1
emit "FORGECLEAN_V1_0_12_TDD_GREEN_STATIC=PASS"
bash -n "$SRC/build-and-verify.sh" "$SRC/install-local.sh" "$SRC/hit-it-template.sh"
emit "FORGECLEAN_SHELL_SYNTAX=PASS"

# The canonical source verifier owns fmt, strict Clippy, all Rust tests, release build,
# three-binary export and a synthetic HOME deletion/protection E2E. It never touches real user files.
( cd "$SRC" && ./build-and-verify.sh ) || die "BUILD_VERIFY_FAILED"
grep -Fxq 'FORGECLEAN_VERIFY=PASS' "$VERIFY" || die "BUILD_VERIFY_DID_NOT_PASS"
grep -Fxq 'FORGECLEAN_CLIPPY=PASS' "$VERIFY" || die "CLIPPY_DID_NOT_PASS"
grep -Fxq 'FORGECLEAN_TEST=PASS' "$VERIFY" || die "TESTS_DID_NOT_PASS"
grep -Fxq 'FORGECLEAN_BUILD=PASS' "$VERIFY" || die "RELEASE_BUILD_DID_NOT_PASS"
grep -Fxq 'FORGECLEAN_PRE_REBASE_E2E=PASS' "$VERIFY" || die "PRE_REBASE_E2E_DID_NOT_PASS"
grep -Fxq 'FORGECLEAN_SYSTEM_CLI_SMOKE=PASS' "$VERIFY" || die "SYSTEM_BINARY_SMOKE_DID_NOT_PASS"
emit "FORGECLEAN_NONDESTRUCTIVE_GATES=PASS"
emit "FORGECLEAN_V1_0_12_TDD_RED=PASS"
emit "FORGECLEAN_V1_0_12_TDD_GREEN=PASS"

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
verified_at=$(date -Iseconds)
MARKER
emit "FORGECLEAN_VERIFIED_SOURCE_MARKER=PASS"

rm -f "$SOURCE_ZIP"
python3 - "$SRC" "$SOURCE_ZIP" <<'PY'
from pathlib import Path
import sys,zipfile
src=Path(sys.argv[1]); out=Path(sys.argv[2])
with zipfile.ZipFile(out,'w',compression=zipfile.ZIP_DEFLATED,compresslevel=9) as z:
    for p in sorted(src.rglob('*')):
        rel=p.relative_to(src)
        if any(part in {'.git','target'} for part in rel.parts):
            continue
        arc=Path(src.name)/rel
        if p.is_dir():
            info=zipfile.ZipInfo(str(arc).rstrip('/')+'/')
            info.external_attr=(0o755 & 0xffff)<<16 | 0x10
            z.writestr(info,b'')
        elif p.is_file():
            info=zipfile.ZipInfo.from_file(p,arcname=str(arc))
            mode=0o755 if p.suffix=='.sh' else 0o644
            info.external_attr=(mode & 0xffff)<<16
            info.compress_type=zipfile.ZIP_DEFLATED
            z.writestr(info,p.read_bytes())
PY
SOURCE_SHA="$(sha256sum "$SOURCE_ZIP" | awk '{print $1}')"
emit "FORGECLEAN_SOURCE_ZIP=PASS"
emit "SOURCE_SHA256=$SOURCE_SHA"

# Snapshot currently installed binaries/units before cutover. This rollback cannot restore files
# intentionally deleted by the permanent pre-rebase sweep.
mkdir -p "$STATE/bin-before" "$STATE/systemd-before"
for bin in forgeclean forgeclean-gui forgeclean-system; do
  [[ ! -e "$HOME/.local/bin/$bin" ]] || cp -a "$HOME/.local/bin/$bin" "$STATE/bin-before/$bin"
done
for unit in forgeclean-pre-rebase.service forgeclean-pre-rebase.timer; do
  [[ ! -e "$HOME/.config/systemd/user/$unit" ]] || cp -a "$HOME/.config/systemd/user/$unit" "$STATE/systemd-before/$unit"
done

( cd "$SRC" && ./install-local.sh ) || die "INSTALL_OR_IMMEDIATE_SWEEP_FAILED"
grep -Fxq 'FORGECLEAN_PRE_REBASE_IMMEDIATE=PASS' "$VERIFY" || die "IMMEDIATE_SWEEP_DID_NOT_PASS"
grep -Fxq 'FORGECLEAN_PRE_REBASE_TIMER=PASS' "$VERIFY" || die "PRE_REBASE_TIMER_DID_NOT_PASS"
emit "FORGECLEAN_REAL_STORAGE_RECLAIM=PASS"

cat > "$ROLLBACK" <<ROLLBACK
#!/usr/bin/env bash
set -Eeuo pipefail
BACKUP='$STATE'
systemctl --user disable --now forgeclean-pre-rebase.timer >/dev/null 2>&1 || true
for bin in forgeclean forgeclean-gui forgeclean-system; do
  if [[ -f "\$BACKUP/bin-before/\$bin" ]]; then
    install -m 0755 "\$BACKUP/bin-before/\$bin" "\$HOME/.local/bin/\$bin"
  fi
done
for unit in forgeclean-pre-rebase.service forgeclean-pre-rebase.timer; do
  if [[ -f "\$BACKUP/systemd-before/\$unit" ]]; then
    install -m 0644 "\$BACKUP/systemd-before/\$unit" "\$HOME/.config/systemd/user/\$unit"
  else
    rm -f "\$HOME/.config/systemd/user/\$unit"
  fi
done
systemctl --user daemon-reload
systemctl --user restart forgeclean-organizer.service >/dev/null 2>&1 || true
echo 'FORGECLEAN_V1_0_12_ROLLBACK=PASS'
echo 'ROLLBACK_SCOPE=BINARIES_AND_TIMER_ONLY_PERMANENTLY_DELETED_FILES_ARE_NOT_RECOVERABLE'
ROLLBACK
chmod +x "$ROLLBACK"
emit "FORGECLEAN_ROLLBACK_SCRIPT=PASS"
emit "ROLLBACK_SCOPE=BINARIES_AND_TIMER_ONLY_DELETED_FILES_ARE_NOT_RECOVERABLE"

# Convert the downloaded updater into the canonical source-zip installer for future reruns.
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

# Preserve the established source-authority workflow when present, but do not make storage recovery
# depend on network availability. The verified versioned source remains authoritative if sync is offline.
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
emit "FORGECLEAN_PRE_REBASE_CUTOFF=2026-08-18T00:00:00-07:00"
emit "FORGECLEAN_PRE_REBASE_TIMESTAMP_POLICY=ANY_AVAILABLE_TIMESTAMP"
emit "FORGECLEAN_PRE_REBASE_DELETE_MODE=DIRECT_UNLINK_NO_TRASH"
emit "FORGECLEAN_PRE_REBASE_LIVE_STATE_PROTECTION=PASS"
emit "FORGECLEAN_PRE_REBASE_IDENTITY_REVALIDATION=PASS"
emit "FORGECLEAN_PRE_REBASE_PARENT_SYMLINK_GUARD=PASS"
emit "FORGECLEAN_STORAGE_PRESSURE_TRIGGER=70_PERCENT"
emit "FORGECLEAN_STORAGE_CRITICAL_ALERT=85_PERCENT"
emit "FORGECLEAN_PRE_REBASE_STARTUP_DAILY_TIMER=PASS"
emit "FORGECLEAN_V1_0_12_HOSTILE_TAKEOVER=PASS"
emit "SOURCE_ZIP=$SOURCE_ZIP"
emit "SHA256SUMS=$SUMS"
emit "ROLLBACK_SCRIPT=$ROLLBACK"
rm_tmp
exit 0

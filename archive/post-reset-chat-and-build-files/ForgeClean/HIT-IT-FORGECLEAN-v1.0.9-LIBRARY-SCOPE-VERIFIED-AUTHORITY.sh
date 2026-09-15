#!/usr/bin/env bash
set -Eeuo pipefail

FIX_VERSION="1.0.9"
REPO_FULL="pattendnd1-png/AETHERFORGE"
HOME_DIR="${HOME:?HOME is required}"
DOWNLOADS="$HOME_DIR/Downloads"
BASE="$DOWNLOADS/ForgeClean-v1.0.5"
TARGET="$DOWNLOADS/ForgeClean-v${FIX_VERSION}"
REPO="$DOWNLOADS/AETHERFORGE"
VERIFY="$DOWNLOADS/ForgeClean-v${FIX_VERSION}-LIBRARY-SCOPE-VERIFIED-AUTHORITY-VERIFY.txt"
STATE_ROOT="$HOME_DIR/.local/state/aetherforge/forgeclean-v${FIX_VERSION}"
BIN_DIR="$HOME_DIR/.local/bin"
ORBIT_STATE="$HOME_DIR/.local/state/aetherforge/orbital-sync"
LOCK_FILE="$ORBIT_STATE/orbital.lock"
ORBIT_WORKER="$HOME_DIR/.local/bin/aetherforge-orbital-sync"
TIMER="aetherforge-orbital-sync.timer"
SERVICE="aetherforge-orbital-sync.service"

mkdir -p "$DOWNLOADS" "$STATE_ROOT" "$BIN_DIR" "$ORBIT_STATE"
: > "$VERIFY"
exec > >(tee -a "$VERIFY") 2>&1

PASS=0
TIMER_WAS_ACTIVE=0
TIMER_WAS_ENABLED=0
LOCK_HELD=0
BACKUP_DIR="$STATE_ROOT/$(date +%Y%m%d-%H%M%S)"
mkdir -p "$BACKUP_DIR"

finish() {
  local rc=$?
  set +e
  if (( LOCK_HELD )); then
    flock -u 9 2>/dev/null || true
  fi
  if (( TIMER_WAS_ENABLED )); then
    systemctl --user enable "$TIMER" >/dev/null 2>&1 || true
  fi
  if (( TIMER_WAS_ACTIVE )); then
    systemctl --user start "$TIMER" >/dev/null 2>&1 || true
  fi
  if (( PASS )); then
    echo "FORGECLEAN_V1_0_9_LIBRARY_SCOPE_VERIFIED_AUTHORITY=PASS"
  else
    echo "FORGECLEAN_V1_0_9_LIBRARY_SCOPE_VERIFIED_AUTHORITY=FAIL:${rc}"
  fi
  echo "VERIFY_FILE=$VERIFY"
  exit "$rc"
}
trap finish EXIT
trap 'echo "ERROR_LINE=$LINENO ERROR_COMMAND=$BASH_COMMAND"' ERR

die() {
  echo "FAIL_REASON=$*"
  exit 1
}

need() {
  command -v "$1" >/dev/null 2>&1 || die "MISSING_COMMAND:$1"
}

version_from_manifest() {
  python3 - "$1" <<'PY'
import pathlib,re,sys
p=pathlib.Path(sys.argv[1])
s=p.read_text()
m=re.search(r'(?m)^version\s*=\s*"([^"]+)"\s*$', s)
print(m.group(1) if m else "")
PY
}

library_scope_check() {
  local root="$1"
  python3 - "$root" <<'PY'
from pathlib import Path
import re, sys
root = Path(sys.argv[1])
src = root / "src"
invalid=[]
allowed=[]
pat=re.compile(r'\bforgeclean::')
for p in sorted(src.rglob("*.rs")):
    rel=p.relative_to(root)
    is_binary = rel.as_posix() in {"src/main.rs", "src/gui_main.rs"} or rel.as_posix().startswith("src/bin/")
    try:
        lines=p.read_text(errors="replace").splitlines()
    except OSError:
        continue
    for no,line in enumerate(lines,1):
        stripped=line.lstrip()
        if stripped.startswith("//"):
            continue
        if pat.search(line):
            item=f"{rel}:{no}:{line.strip()}"
            (allowed if is_binary else invalid).append(item)
print(f"LIBRARY_SCOPE_INVALID={len(invalid)}")
for item in invalid:
    print(f"LIBRARY_SCOPE_INVALID_ITEM={item}")
print(f"BINARY_SCOPE_ALLOWED={len(allowed)}")
for item in allowed[:40]:
    print(f"BINARY_SCOPE_ALLOWED_ITEM={item}")
sys.exit(1 if invalid else 0)
PY
}

patch_root_cause() {
  python3 - "$1/src/orbital_ui.rs" <<'PY'
from pathlib import Path
import sys
p=Path(sys.argv[1])
s=p.read_text()
old1="use forgeclean::orbital::{"
old2="use forgeclean::orbital_monitor_model::{"
count=s.count(old1)+s.count(old2)
if count < 2:
    raise SystemExit(f"expected both self-crate imports in {p}, found {count}")
s=s.replace(old1,"use crate::orbital::{")
s=s.replace(old2,"use crate::orbital_monitor_model::{")
p.write_text(s)
print("ROOT_CAUSE_PATCH_REPLACEMENTS=2")
PY
}

bump_version() {
  python3 - "$1" "$FIX_VERSION" <<'PY'
from pathlib import Path
import re,sys
root=Path(sys.argv[1]); ver=sys.argv[2]
p=root/"Cargo.toml"
s=p.read_text()
s2,n=re.subn(r'(?m)^version\s*=\s*"[^"]+"\s*$', f'version = "{ver}"', s, count=1)
if n != 1:
    raise SystemExit("Cargo.toml package version not found uniquely")
p.write_text(s2)
lock=root/"Cargo.lock"
if lock.exists():
    t=lock.read_text()
    # Change only forgeclean's package stanza.
    pattern=r'(?ms)(\[\[package\]\]\nname = "forgeclean"\nversion = ")[^"]+("\n)'
    t2,n2=re.subn(pattern, rf'\g<1>{ver}\2', t, count=1)
    if n2:
        lock.write_text(t2)
print(f"VERSION_BUMP={ver}")
PY
}

echo "FORGECLEAN_V1_0_9_LIBRARY_SCOPE_VERIFIED_AUTHORITY=START"
echo "SOURCE_BASELINE=$BASE"
echo "SOURCE_TARGET=$TARGET"
echo "REPO=$REPO_FULL"
echo "FIX_VERSION=$FIX_VERSION"

for c in git gh rsync python3 sha256sum flock systemctl cargo rustc; do need "$c"; done

[[ -f "$BASE/Cargo.toml" ]] || die "BASELINE_NOT_FOUND:$BASE"
BASE_VER="$(version_from_manifest "$BASE/Cargo.toml")"
[[ "$BASE_VER" == "1.0.5" ]] || die "BASELINE_VERSION_EXPECTED_1.0.5_GOT_$BASE_VER"
echo "BASELINE_VERSION=PASS"

# The exact root-cause signature must still exist in the pristine failed baseline.
grep -Fq 'use forgeclean::orbital::{' "$BASE/src/orbital_ui.rs" || die "ROOT_CAUSE_SIGNATURE_MISSING_ORBITAL"
grep -Fq 'use forgeclean::orbital_monitor_model::{' "$BASE/src/orbital_ui.rs" || die "ROOT_CAUSE_SIGNATURE_MISSING_MONITOR_MODEL"
echo "ROOT_CAUSE_SIGNATURE=CONFIRMED"

# Preserve proof that package-name imports are legitimate in binary crates.
if grep -Rqs --include='*.rs' 'forgeclean::' "$BASE/src/main.rs" "$BASE/src/gui_main.rs" "$BASE/src/bin" 2>/dev/null; then
  echo "BINARY_CRATE_IMPORT_PATTERN=CONFIRMED"
else
  echo "BINARY_CRATE_IMPORT_PATTERN=NOT_PRESENT"
fi

[[ -d "$REPO/.git" ]] || die "REPO_NOT_FOUND:$REPO"

git -C "$REPO" fetch origin main --quiet
LOCAL_HEAD_BEFORE="$(git -C "$REPO" rev-parse HEAD)"
REMOTE_HEAD_BEFORE="$(git -C "$REPO" rev-parse origin/main)"
echo "LOCAL_HEAD_BEFORE=$LOCAL_HEAD_BEFORE"
echo "REMOTE_HEAD_BEFORE=$REMOTE_HEAD_BEFORE"
[[ "$LOCAL_HEAD_BEFORE" == "$REMOTE_HEAD_BEFORE" ]] || die "REPO_NOT_ALIGNED"

systemctl --user is-active --quiet "$TIMER" && TIMER_WAS_ACTIVE=1 || true
systemctl --user is-enabled --quiet "$TIMER" && TIMER_WAS_ENABLED=1 || true
systemctl --user stop "$TIMER" >/dev/null 2>&1 || true
systemctl --user stop "$SERVICE" >/dev/null 2>&1 || true
echo "SYNC_AUTHORITIES_QUIESCED=PASS"

exec 9>"$LOCK_FILE"
flock -w 90 9 || die "ORBITAL_LOCK_TIMEOUT"
LOCK_HELD=1
echo "ORBITAL_SHARED_LOCK=ACQUIRED"

# Dirty-state check intentionally happens only after scheduler quiesce + shared lock.
DIRTY_BEFORE="$(git -C "$REPO" status --porcelain=v1 -uall | wc -l | tr -d ' ')"
echo "DIRTY_BEFORE=$DIRTY_BEFORE"
if [[ "$DIRTY_BEFORE" != "0" ]]; then
  echo "DIRTY_SAMPLE=START"
  git -C "$REPO" status --short | head -80 || true
  echo "DIRTY_SAMPLE=END"
  die "REPO_DIRTY_AFTER_QUIESCE"
fi

if [[ -e "$TARGET" ]]; then
  mv "$TARGET" "$BACKUP_DIR/ForgeClean-v${FIX_VERSION}.preexisting"
  echo "PREEXISTING_TARGET_PRESERVED=$BACKUP_DIR/ForgeClean-v${FIX_VERSION}.preexisting"
fi
mkdir -p "$TARGET"
rsync -a --delete --exclude target --exclude .git "$BASE/" "$TARGET/"
echo "VERSIONED_SOURCE_COPY=PASS"

mkdir -p "$TARGET/tools"
cat > "$TARGET/tools/verify-library-import-scope.py" <<'PY'
#!/usr/bin/env python3
from pathlib import Path
import re, sys
root = Path(__file__).resolve().parents[1]
src = root / "src"
invalid=[]
allowed=[]
pat=re.compile(r'\bforgeclean::')
for p in sorted(src.rglob("*.rs")):
    rel=p.relative_to(root)
    is_binary = rel.as_posix() in {"src/main.rs", "src/gui_main.rs"} or rel.as_posix().startswith("src/bin/")
    lines=p.read_text(errors="replace").splitlines()
    for no,line in enumerate(lines,1):
        if line.lstrip().startswith("//"):
            continue
        if pat.search(line):
            item=f"{rel}:{no}:{line.strip()}"
            (allowed if is_binary else invalid).append(item)
print(f"LIBRARY_SCOPE_INVALID={len(invalid)}")
for item in invalid:
    print(f"LIBRARY_SCOPE_INVALID_ITEM={item}")
print(f"BINARY_SCOPE_ALLOWED={len(allowed)}")
for item in allowed[:40]:
    print(f"BINARY_SCOPE_ALLOWED_ITEM={item}")
sys.exit(1 if invalid else 0)
PY
chmod +x "$TARGET/tools/verify-library-import-scope.py"

# RED: the verifier must reject the known bad library imports before production repair.
set +e
RED_OUT="$(cd "$TARGET" && ./tools/verify-library-import-scope.py 2>&1)"
RED_RC=$?
set -e
printf '%s\n' "$RED_OUT"
[[ "$RED_RC" != "0" ]] || die "TDD_RED_UNEXPECTED_PASS"
grep -Fq 'src/orbital_ui.rs' <<<"$RED_OUT" || die "TDD_RED_WRONG_FAILURE"
echo "TDD_RED=PASS"

patch_root_cause "$TARGET"
echo "ROOT_CAUSE_PATCH=PASS"

# GREEN: library scope must now be clean while binary imports remain explicitly allowed.
(cd "$TARGET" && ./tools/verify-library-import-scope.py)
echo "TDD_GREEN=PASS"

# Explicitly prove we did not rewrite legitimate binary crate imports.
if grep -Rqs --include='*.rs' 'forgeclean::' "$TARGET/src/main.rs" "$TARGET/src/gui_main.rs" "$TARGET/src/bin" 2>/dev/null; then
  echo "BINARY_IMPORTS_PRESERVED=PASS"
else
  echo "BINARY_IMPORTS_PRESERVED=NOT_PRESENT"
fi

bump_version "$TARGET"
[[ "$(version_from_manifest "$TARGET/Cargo.toml")" == "$FIX_VERSION" ]] || die "VERSION_BUMP_FAILED"
echo "TARGET_VERSION=PASS"

# No stale package-name self imports may remain anywhere that compiles as the library.
library_scope_check "$TARGET"
echo "LIBRARY_SCOPE_REGRESSION=PASS"

cd "$TARGET"
cargo fmt --all -- --check
echo "FORGECLEAN_FMT=PASS"

cargo clippy --workspace --all-targets --all-features -- -D warnings
echo "FORGECLEAN_CLIPPY=PASS"

cargo test --workspace --all-features
echo "FORGECLEAN_TEST=PASS"

cargo build --workspace --all-targets --all-features --release
echo "FORGECLEAN_RELEASE_BUILD=PASS"

for b in forgeclean forgeclean-gui forgeclean-system; do
  [[ -x "$TARGET/target/release/$b" ]] || die "MISSING_RELEASE_BINARY:$b"
  if [[ -e "$BIN_DIR/$b" ]]; then
    cp -a "$BIN_DIR/$b" "$BACKUP_DIR/$b.before"
  fi
  install -m 0755 "$TARGET/target/release/$b" "$BIN_DIR/$b"
done
echo "BINARY_CUTOVER=PASS"

# CLI smoke without launching GUI.
CLI_VER="$($BIN_DIR/forgeclean-system version 2>&1 || true)"
printf '%s\n' "$CLI_VER"
grep -Fq "$FIX_VERSION" <<<"$CLI_VER" || die "CLI_VERSION_SMOKE_FAILED"
echo "CLI_VERSION_SMOKE=PASS"

sha256sum "$TARGET/target/release/forgeclean" "$TARGET/target/release/forgeclean-gui" "$TARGET/target/release/forgeclean-system" > "$DOWNLOADS/ForgeClean-v${FIX_VERSION}-SHA256SUMS.txt"
echo "SHA256SUMS=$DOWNLOADS/ForgeClean-v${FIX_VERSION}-SHA256SUMS.txt"

# Release the shared lock before invoking the authoritative full orbit ourselves.
flock -u 9
LOCK_HELD=0
echo "ORBITAL_SHARED_LOCK=RELEASED_FOR_FULL_SYNC"

if [[ -x "$ORBIT_WORKER" ]]; then
  "$ORBIT_WORKER" full
  echo "AUTHORITATIVE_FULL_ORBIT=PASS"
else
  die "ORBITAL_WORKER_NOT_FOUND:$ORBIT_WORKER"
fi

# Verify the full orbit selected v1.0.9 as ForgeClean source authority.
MAP="$REPO/migration/SOURCE-MAP.tsv"
if [[ ! -f "$MAP" ]]; then
  MAP="$REPO/artifacts/SOURCE-MAP.tsv"
fi
if [[ -f "$MAP" ]]; then
  grep -F $'ForgeClean\t' "$MAP" | tail -1 || true
fi

git -C "$REPO" fetch origin main --quiet
LOCAL_HEAD_AFTER="$(git -C "$REPO" rev-parse HEAD)"
REMOTE_HEAD_AFTER="$(git -C "$REPO" rev-parse origin/main)"
DIRTY_AFTER="$(git -C "$REPO" status --porcelain=v1 -uall | wc -l | tr -d ' ')"
echo "LOCAL_HEAD_AFTER=$LOCAL_HEAD_AFTER"
echo "REMOTE_HEAD_AFTER=$REMOTE_HEAD_AFTER"
echo "DIRTY_AFTER=$DIRTY_AFTER"
[[ "$LOCAL_HEAD_AFTER" == "$REMOTE_HEAD_AFTER" ]] || die "REMOTE_PARITY_FAILED"
[[ "$DIRTY_AFTER" == "0" ]] || die "REPO_DIRTY_AFTER_FULL_ORBIT"
echo "REMOTE_PARITY=PASS"

[[ -f "$REPO/apps/ForgeClean/Cargo.toml" ]] || die "GITHUB_STAGING_SOURCE_MISSING"
[[ "$(version_from_manifest "$REPO/apps/ForgeClean/Cargo.toml")" == "$FIX_VERSION" ]] || die "GITHUB_STAGING_VERSION_NOT_$FIX_VERSION"
echo "REPO_FORGECLEAN_VERSION=PASS"

REMOTE_MANIFEST="$(gh api "repos/$REPO_FULL/contents/apps/ForgeClean/Cargo.toml?ref=main" --jq '.content' | tr -d '\n' | base64 -d)"
grep -Fq "version = \"$FIX_VERSION\"" <<<"$REMOTE_MANIFEST" || die "GITHUB_REMOTE_VERSION_NOT_$FIX_VERSION"
echo "GITHUB_FORGECLEAN_SOURCE=PASS"

# Immediate fast refresh verifies the normal scheduler path after the full orbit.
"$ORBIT_WORKER" fast
echo "FAST_ORBIT_REFRESH=PASS"

STATUS="$DOWNLOADS/AETHERFORGE-ORBITAL-SYNC-STATUS.txt"
[[ -f "$STATUS" ]] || die "ORBITAL_STATUS_MISSING"
grep -Fq 'result=PASS' "$STATUS" || die "ORBITAL_STATUS_NOT_PASS"
grep -Fq 'queue_ahead=0' "$STATUS" || die "ORBITAL_QUEUE_AHEAD_NONZERO"
grep -Fq 'queue_behind=0' "$STATUS" || die "ORBITAL_QUEUE_BEHIND_NONZERO"
grep -Fq 'dirty_count=0' "$STATUS" || die "ORBITAL_DIRTY_NONZERO"
echo "ORBITAL_STATUS=PASS"

PASS=1
exit 0

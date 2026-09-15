#!/usr/bin/env bash
set -Eeuo pipefail

BASE_VERSION="1.0.7"
VERSION="1.0.10"

REPO_DIR="${AETHERFORGE_REPO_DIR:-$HOME/Downloads/AETHERFORGE}"
REPO_FULL="${AETHERFORGE_REPO_FULL:-pattendnd1-png/AETHERFORGE}"
BASE_SRC="$REPO_DIR/apps/ForgeClean"
SRC="$HOME/Downloads/ForgeClean-v${VERSION}"

SWEEP="${AETHERFORGE_FULL_SWEEP_BIN:-$HOME/.local/bin/aetherforge-postreset-sweep}"
CANONICAL_SWEEP="$REPO_DIR/os/services/orbital-sync/aetherforge-postreset-sweep"
ORBIT="${AETHERFORGE_ORBITAL_BIN:-$HOME/.local/bin/aetherforge-orbital-sync}"
MAP="${XDG_CONFIG_HOME:-$HOME/.config}/aetherforge/postreset-source-map.tsv"

TIMER="aetherforge-orbital-sync.timer"
LEGACY_AUTO_TIMER="aetherforge-git-autosync.timer"
LEGACY_SWEEP_TIMER="aetherforge-postreset-sweep.timer"
LOCK_FILE="${AETHERFORGE_ORBITAL_STATE_DIR:-$HOME/.local/state/aetherforge/orbital-sync}/orbital.lock"

VERIFY="$HOME/Downloads/ForgeClean-v${VERSION}-ATOMIC-ORBIT-RECOVERY-VERIFY.txt"
SOURCE_ZIP="$HOME/Downloads/ForgeClean-v${VERSION}-SOURCE.zip"
SUMS="$HOME/Downloads/ForgeClean-v${VERSION}-SHA256SUMS.txt"
ROLLBACK="$HOME/Downloads/ForgeClean-v${VERSION}-ROLLBACK.sh"
STATE="$HOME/.local/state/forgeclean-v${VERSION}-$(date +%Y%m%d-%H%M%S)"
VERIFIED_MARKER=".aetherforge-verified-source"

mkdir -p "$STATE" "$(dirname "$LOCK_FILE")"
: > "$VERIFY"

emit(){ printf '%s\n' "$*" | tee -a "$VERIFY"; }
die(){ emit "FORGECLEAN_V1_0_10_ATOMIC_ORBIT_RECOVERY=FAIL:$*"; exit 1; }

timer_active=0
timer_enabled=0
lock_open=0

cleanup(){
  local rc=$?
  trap - EXIT
  if (( lock_open == 1 )); then
    flock -u 9 >/dev/null 2>&1 || true
    exec 9>&- || true
  fi

  systemctl --user disable --now "$LEGACY_AUTO_TIMER" >/dev/null 2>&1 || true
  systemctl --user disable --now "$LEGACY_SWEEP_TIMER" >/dev/null 2>&1 || true
  systemctl --user stop aetherforge-git-autosync.service >/dev/null 2>&1 || true
  systemctl --user stop aetherforge-postreset-sweep.service >/dev/null 2>&1 || true

  if (( timer_enabled == 1 )); then systemctl --user enable "$TIMER" >/dev/null 2>&1 || true; fi
  if (( timer_active == 1 )); then systemctl --user start "$TIMER" >/dev/null 2>&1 || true; fi

  if (( rc == 0 )); then
    emit "FORGECLEAN_V1_0_10_ATOMIC_ORBIT_RECOVERY=PASS"
  else
    emit "FORGECLEAN_V1_0_10_ATOMIC_ORBIT_RECOVERY=FAIL:${rc}"
  fi
  emit "VERIFY_FILE=$VERIFY"
  exit "$rc"
}
trap cleanup EXIT

for cmd in cargo rustc git gh rsync python3 sha256sum flock systemctl grep awk base64 find; do
  command -v "$cmd" >/dev/null 2>&1 || die "MISSING_COMMAND:$cmd"
done

[[ -d "$REPO_DIR/.git" ]] || die "AETHERFORGE_REPO_MISSING:$REPO_DIR"
[[ -x "$SWEEP" ]] || die "SWEEP_WORKER_MISSING:$SWEEP"
[[ -x "$ORBIT" ]] || die "ORBITAL_WORKER_MISSING:$ORBIT"

emit "FORGECLEAN_V1_0_10_ATOMIC_ORBIT_RECOVERY=START"
emit "REPO=$REPO_FULL"
emit "SOURCE_BASELINE=$BASE_SRC"
emit "SOURCE_TARGET=$SRC"

systemctl --user is-active --quiet "$TIMER" && timer_active=1 || true
systemctl --user is-enabled --quiet "$TIMER" && timer_enabled=1 || true
systemctl --user stop "$TIMER" >/dev/null 2>&1 || true
systemctl --user disable --now "$LEGACY_AUTO_TIMER" >/dev/null 2>&1 || true
systemctl --user disable --now "$LEGACY_SWEEP_TIMER" >/dev/null 2>&1 || true
systemctl --user stop aetherforge-git-autosync.service >/dev/null 2>&1 || true
systemctl --user stop aetherforge-postreset-sweep.service >/dev/null 2>&1 || true
emit "SYNC_AUTHORITIES_QUIESCED=PASS"

exec 9>"$LOCK_FILE"
flock -w 30 9 || die "ORBITAL_LOCK_TIMEOUT"
lock_open=1
emit "ORBITAL_SHARED_LOCK=ACQUIRED"

git -C "$REPO_DIR" fetch origin main --quiet
LOCAL_BEFORE="$(git -C "$REPO_DIR" rev-parse HEAD)"
REMOTE_BEFORE="$(git -C "$REPO_DIR" rev-parse origin/main)"
emit "LOCAL_HEAD_BEFORE=$LOCAL_BEFORE"
emit "REMOTE_HEAD_BEFORE=$REMOTE_BEFORE"
[[ "$LOCAL_BEFORE" == "$REMOTE_BEFORE" ]] || die "REPO_NOT_ALIGNED"

DELETED_FILE="$STATE/deleted-paths.zlist"
NONDELETE_FILE="$STATE/nondelete-paths.zlist"
UNTRACKED_FILE="$STATE/untracked-paths.zlist"
STAGED_FILE="$STATE/staged-paths.zlist"

git -C "$REPO_DIR" diff --name-only --diff-filter=D -z > "$DELETED_FILE"
git -C "$REPO_DIR" diff --name-only --diff-filter=ACMRTUXB -z > "$NONDELETE_FILE"
git -C "$REPO_DIR" ls-files --others --exclude-standard -z > "$UNTRACKED_FILE"
git -C "$REPO_DIR" diff --cached --name-only -z > "$STAGED_FILE"

count_z() {
  python3 - "$1" <<'PY'
from pathlib import Path
import sys
b=Path(sys.argv[1]).read_bytes()
print(sum(1 for x in b.split(b"\0") if x))
PY
}

DELETED_COUNT="$(count_z "$DELETED_FILE")"
NONDELETE_COUNT="$(count_z "$NONDELETE_FILE")"
UNTRACKED_COUNT="$(count_z "$UNTRACKED_FILE")"
STAGED_COUNT="$(count_z "$STAGED_FILE")"

emit "DIRTY_DELETED=$DELETED_COUNT"
emit "DIRTY_NONDELETE=$NONDELETE_COUNT"
emit "DIRTY_UNTRACKED=$UNTRACKED_COUNT"
emit "DIRTY_STAGED=$STAGED_COUNT"

[[ "$NONDELETE_COUNT" == "0" ]] || die "DIRTY_TREE_HAS_NONDELETION_WORK"
[[ "$UNTRACKED_COUNT" == "0" ]] || die "DIRTY_TREE_HAS_UNTRACKED_WORK"
[[ "$STAGED_COUNT" == "0" ]] || die "DIRTY_TREE_HAS_STAGED_WORK"

if (( DELETED_COUNT > 0 )); then
  git -C "$REPO_DIR" restore --worktree --source=HEAD \
    --pathspec-from-file="$DELETED_FILE" --pathspec-file-nul
  emit "DELETION_ONLY_WORKTREE_RESTORE=PASS"
else
  emit "DELETION_ONLY_WORKTREE_RESTORE=NOT_NEEDED"
fi

DIRTY_AFTER_RESTORE="$(git -C "$REPO_DIR" status --porcelain | wc -l | tr -d ' ')"
emit "DIRTY_AFTER_RESTORE=$DIRTY_AFTER_RESTORE"
[[ "$DIRTY_AFTER_RESTORE" == "0" ]] || die "WORKTREE_NOT_CLEAN_AFTER_SAFE_RESTORE"
emit "WORKTREE_RECOVERY=PASS"

cp -a "$SWEEP" "$STATE/aetherforge-postreset-sweep.before"

python3 - "$SWEEP" <<'PY'
from pathlib import Path
import sys

p=Path(sys.argv[1])
s=p.read_text()

start=s.find("safe_rsync_tree() {")
end=s.find("\nis_source_root() {", start)
if start < 0 or end < 0:
    raise SystemExit("safe_rsync_tree block anchors missing")

new = r'''safe_rsync_tree() {
  local src="$1" dst="$2"
  local parent base stage src_files dst_files
  [[ -d "$src" ]] || { log "SOURCE_IMPORT_SOURCE_MISSING=$src"; return 70; }
  find "$src" -mindepth 1 -maxdepth 1 -print -quit 2>/dev/null | grep -q . || {
    log "SOURCE_IMPORT_SOURCE_EMPTY=$src"
    return 71
  }

  parent="$(dirname -- "$dst")"
  base="$(basename -- "$dst")"
  mkdir -p "$parent"
  stage="$(mktemp -d "$parent/.${base}.stage.XXXXXX")"

  if ! rsync -a \
    --exclude='.git/' --exclude='.hg/' --exclude='.svn/' \
    --exclude='target/' --exclude='node_modules/' --exclude='dist/' --exclude='build/' \
    --exclude='out/' --exclude='.next/' --exclude='.cache/' --exclude='__pycache__/' \
    --exclude='packaging/arch/pkg/' --exclude='packaging/arch/src/' \
    --exclude='pkgdest/' --exclude='srcdest/' --exclude='srcpkgdest/' \
    --exclude='.venv/' --exclude='venv/' --exclude='.tox/' --exclude='.pytest_cache/' \
    --exclude='.env' --exclude='.env.*' --exclude='*.pem' --exclude='*.key' --exclude='*.p12' \
    --exclude='id_rsa*' --exclude='id_ed25519*' --exclude='credentials*' --exclude='secrets*' \
    --exclude='*.iso' --exclude='*.img' --exclude='*.qcow2' --exclude='*.vdi' --exclude='*.vmdk' \
    --exclude='*.pkg.tar.zst' --exclude='*.AppImage' \
    "$src" "$stage/"
  then
    rm -rf -- "$stage"
    log "SOURCE_IMPORT_STAGE_RSYNC=FAIL:$src"
    return 72
  fi

  src_files="$(find "$stage" -type f 2>/dev/null | wc -l | tr -d ' ')"
  dst_files=0
  [[ -d "$dst" ]] && dst_files="$(find "$dst" -type f 2>/dev/null | wc -l | tr -d ' ')"
  [[ "$src_files" =~ ^[0-9]+$ ]] || src_files=0
  [[ "$dst_files" =~ ^[0-9]+$ ]] || dst_files=0

  if (( src_files == 0 )); then
    rm -rf -- "$stage"
    log "SOURCE_IMPORT_STAGE_EMPTY=$src"
    return 73
  fi

  if (( dst_files >= 100 && src_files * 4 < dst_files )); then
    log "SOURCE_IMPORT_SHRINK_GUARD=$src_files/$dst_files:$src"
    rm -rf -- "$stage"
    return 74
  fi

  if [[ -e "$dst" ]]; then
    if ! python3 - "$stage" "$dst" <<'PYSWAP'
import ctypes, os, sys
stage=os.fsencode(sys.argv[1]); dst=os.fsencode(sys.argv[2])
libc=ctypes.CDLL(None, use_errno=True)
fn=getattr(libc, "renameat2", None)
if fn is None:
    raise SystemExit("renameat2 unavailable")
fn.argtypes=[ctypes.c_int, ctypes.c_char_p, ctypes.c_int, ctypes.c_char_p, ctypes.c_uint]
fn.restype=ctypes.c_int
AT_FDCWD=-100
RENAME_EXCHANGE=2
rc=fn(AT_FDCWD, stage, AT_FDCWD, dst, RENAME_EXCHANGE)
if rc != 0:
    err=ctypes.get_errno()
    raise OSError(err, os.strerror(err))
PYSWAP
    then
      rm -rf -- "$stage"
      log "SOURCE_IMPORT_ATOMIC_SWAP=FAIL:$dst"
      return 75
    fi
    rm -rf -- "$stage" || true
  else
    mv -- "$stage" "$dst"
  fi

  log "SOURCE_IMPORT_ATOMIC_SWAP=PASS:$base:$src_files"
}
'''

s=s[:start]+new+s[end:]

fc_start=s.find("ensure_forgeclean_source_mapping() {")
fc_end=s.find("\ncapture_garuda_base() {", fc_start)
if fc_start < 0 or fc_end < 0:
    raise SystemExit("ForgeClean authority block missing")
head, block, tail=s[:fc_start], s[fc_start:fc_end], s[fc_end:]

marker=".aetherforge-verified-source"
if marker not in block:
    needle=(
        "    ver=tuple(map(int,m_ver.groups()))\n"
        "    candidates.append((ver, d.stat().st_mtime_ns, d))"
    )
    repl=(
        "    ver=tuple(map(int,m_ver.groups()))\n"
        f'    marker=d/"{marker}"\n'
        "    if not marker.is_file():\n"
        "        continue\n"
        '    marker_text=marker.read_text(errors="ignore")\n'
        "    m_marker=re.search(r'(?m)^version=(\\d+)\\.(\\d+)\\.(\\d+)$', marker_text)\n"
        "    if not m_marker or tuple(map(int,m_marker.groups())) != ver:\n"
        "        continue\n"
        "    candidates.append((ver, d.stat().st_mtime_ns, d))"
    )
    if needle not in block:
        raise SystemExit("ForgeClean candidate anchor missing")
    block=block.replace(needle,repl,1)

old='  elif [[ -f "$REPO_DIR/apps/ForgeClean/Cargo.toml" ]]; then'
new=f'  elif [[ -f "$REPO_DIR/apps/ForgeClean/Cargo.toml" && -f "$REPO_DIR/apps/ForgeClean/{marker}" ]]; then'
if old in block:
    block=block.replace(old,new,1)
elif new not in block:
    raise SystemExit("ForgeClean fallback anchor missing")

p.write_text(head+block+tail)
PY

chmod 0755 "$SWEEP"
bash -n "$SWEEP"
grep -Fq 'SOURCE_IMPORT_ATOMIC_SWAP=PASS' "$SWEEP" || die "ATOMIC_IMPORT_PATCH_MISSING"
grep -Fq 'SOURCE_IMPORT_SHRINK_GUARD=' "$SWEEP" || die "SHRINK_GUARD_PATCH_MISSING"
grep -Fq "$VERIFIED_MARKER" "$SWEEP" || die "VERIFIED_AUTHORITY_PATCH_MISSING"
emit "ATOMIC_IMPORT_PATCH=PASS"
emit "SOURCE_SHRINK_GUARD=PASS"
emit "VERIFIED_SOURCE_AUTHORITY_PATCH=PASS"

mkdir -p "$(dirname "$CANONICAL_SWEEP")"
cp -f "$SWEEP" "$CANONICAL_SWEEP"
chmod 0755 "$CANONICAL_SWEEP"

git -C "$REPO_DIR" add -- os/services/orbital-sync/aetherforge-postreset-sweep
if ! git -C "$REPO_DIR" diff --cached --quiet; then
  git -C "$REPO_DIR" \
    -c user.name='AETHERFORGE Automation' \
    -c user.email='actions@users.noreply.github.com' \
    commit -m "Make orbital source imports atomic and shrink-safe" >>"$VERIFY" 2>&1
  git -C "$REPO_DIR" push origin HEAD:main >>"$VERIFY" 2>&1
  emit "ATOMIC_SWEEP_CANONICAL_PUSH=PASS"
else
  emit "ATOMIC_SWEEP_CANONICAL_PUSH=ALREADY_CURRENT"
fi

[[ -f "$BASE_SRC/Cargo.toml" ]] || die "CANONICAL_FORGECLEAN_MISSING"
BASE_CARGO_VERSION="$(awk -F= '/^[[:space:]]*version[[:space:]]*=/{gsub(/["[:space:]]/,"",$2); print $2; exit}' "$BASE_SRC/Cargo.toml")"
[[ "$BASE_CARGO_VERSION" == "$BASE_VERSION" ]] || die "CANONICAL_BASE_VERSION_MISMATCH:$BASE_CARGO_VERSION"
emit "CANONICAL_BASE_VERSION=PASS"

grep -Fq 'use crate::orbital::{local_storage_summary, read_status};' "$BASE_SRC/src/orbital_ui.rs" \
  || die "CANONICAL_ORBITAL_IMPORT_NOT_REPAIRED"
grep -Fq 'use crate::orbital_monitor_model::' "$BASE_SRC/src/orbital_ui.rs" \
  || die "CANONICAL_MONITOR_IMPORT_NOT_REPAIRED"
emit "CANONICAL_LIBRARY_IMPORTS=PASS"

if [[ -e "$SRC" ]]; then
  mv "$SRC" "$STATE/ForgeClean-v${VERSION}.preexisting"
  emit "PREEXISTING_V1_0_10_QUARANTINED=PASS"
fi

mkdir -p "$SRC"
rsync -a --exclude='.git/' --exclude='target/' --exclude="$VERIFIED_MARKER" "$BASE_SRC/" "$SRC/"
emit "FORGECLEAN_SOURCE_COPY=PASS"

cat > "$SRC/tests/regression_v1_0_10_atomic_orbit.sh" <<'TEST'
#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

grep -Eq '^version[[:space:]]*=[[:space:]]*"1\.0\.10"' Cargo.toml
grep -Fq 'use crate::orbital::{local_storage_summary, read_status};' src/orbital_ui.rs
grep -Fq 'use crate::orbital_monitor_model::' src/orbital_ui.rs
grep -Fq 'pub mod orbital_ui;' src/lib.rs
grep -Fq 'Self::Orbital => "Orbital Sync"' src/gui.rs
grep -Fq 'Page::Orbital => crate::orbital_ui::show(ui)' src/gui.rs

python3 - . <<'PY'
from pathlib import Path
import sys
root=Path(sys.argv[1])
bad=[]
for p in root.joinpath("src").rglob("*.rs"):
    rel=p.relative_to(root/"src")
    if rel in (Path("main.rs"),Path("gui_main.rs")) or (rel.parts and rel.parts[0]=="bin"):
        continue
    for i,line in enumerate(p.read_text(errors="ignore").splitlines(),1):
        if line.lstrip().startswith("use forgeclean::"):
            bad.append(f"{rel}:{i}:{line}")
if bad:
    print("\n".join(bad))
    raise SystemExit(1)
PY

grep -Fq 'use forgeclean::' src/main.rs
grep -Fq 'use forgeclean::' src/gui_main.rs
grep -Fq 'use forgeclean::' src/bin/forgeclean-system.rs

echo 'FORGECLEAN_V1_0_10_ATOMIC_ORBIT_REGRESSION=PASS'
TEST
chmod +x "$SRC/tests/regression_v1_0_10_atomic_orbit.sh"

if "$SRC/tests/regression_v1_0_10_atomic_orbit.sh" >>"$VERIFY" 2>&1; then
  die "TDD_RED_UNEXPECTEDLY_PASSED"
else
  emit "TDD_RED=PASS"
fi

python3 - "$SRC" <<'PY'
from pathlib import Path
import re, sys
root=Path(sys.argv[1])

p=root/"Cargo.toml"
s=p.read_text()
s,n=re.subn(r'(?m)^(version\s*=\s*)"1\.0\.7"\s*$',r'\g<1>"1.0.10"',s,count=1)
if n != 1:
    raise SystemExit("Cargo 1.0.7 version anchor missing")
p.write_text(s)

for name in ("build-and-verify.sh","install-local.sh","hit-it-template.sh"):
    p=root/name
    if not p.is_file():
        continue
    s=p.read_text()
    s=re.sub(r'(?m)^VERSION="1\.0\.7"$','VERSION="1.0.10"',s,count=1)
    s=re.sub(r'(?m)^VERSION="1\.0\.5"$','VERSION="1.0.10"',s,count=1)
    if name=="build-and-verify.sh":
        gate='run_gate FORGECLEAN_V1_0_10_ATOMIC_ORBIT ./tests/regression_v1_0_10_atomic_orbit.sh || status=1\n'
        if gate not in s:
            anchor='run_gate FORGECLEAN_V1_0_5_ORBITAL_UI ./tests/regression_v1_0_5_orbital_ui.sh || status=1\n'
            if anchor not in s:
                raise SystemExit("build gate anchor missing")
            s=s.replace(anchor,anchor+gate,1)
    p.write_text(s)

p=root/"README.md"
s=p.read_text()
s=re.sub(r'(?m)^# ForgeClean v1\.0\.7$','# ForgeClean v1.0.10',s,count=1)
marker="## v1.0.10 — Atomic Orbital Recovery\n"
if marker not in s:
    ins=(
        "\n## v1.0.10 — Atomic Orbital Recovery\n\n"
        "- Recovers deletion-only interrupted orbital worktrees from canonical HEAD.\n"
        "- Uses staged + atomic project source imports instead of in-place destructive rsync.\n"
        "- Rejects catastrophic source shrink before canonical app replacement.\n"
        "- Requires verified ForgeClean source markers and preserves valid binary-crate imports.\n\n"
    )
    pos=s.find("\n")
    s=s[:pos+1]+ins+s[pos+1:]
p.write_text(s)
PY
emit "FORGECLEAN_VERSION_PATCH=PASS"

"$SRC/tests/regression_v1_0_10_atomic_orbit.sh" >>"$VERIFY" 2>&1
emit "TDD_GREEN=PASS"

cd "$SRC"
cargo fmt --all >>"$VERIFY" 2>&1
cargo fmt --all -- --check >>"$VERIFY" 2>&1
emit "FORGECLEAN_FMT=PASS"

cargo clippy --all-targets --all-features -- -D warnings >>"$VERIFY" 2>&1
emit "FORGECLEAN_CLIPPY=PASS"

cargo test --all-targets --all-features >>"$VERIFY" 2>&1
emit "FORGECLEAN_TEST=PASS"

cargo build --release --all-features >>"$VERIFY" 2>&1
emit "FORGECLEAN_BUILD=PASS"

for bin in forgeclean forgeclean-gui forgeclean-system; do
  [[ -x "$SRC/target/release/$bin" ]] || die "MISSING_RELEASE_BINARY:$bin"
done
emit "THREE_BINARY_GATE=PASS"

"$SRC/target/release/forgeclean-gui" --self-test >>"$VERIFY" 2>&1
emit "FORGECLEAN_GUI_SELF_TEST=PASS"

GUI_VERSION_OUT="$("$SRC/target/release/forgeclean-gui" --version 2>&1)"
printf '%s\n' "$GUI_VERSION_OUT" >>"$VERIFY"
grep -Fq "v${VERSION}" <<<"$GUI_VERSION_OUT" || die "GUI_VERSION_SMOKE_FAILED"
emit "FORGECLEAN_GUI_VERSION=PASS"

"$SRC/target/release/forgeclean-system" orbital status >>"$VERIFY" 2>&1
emit "FORGECLEAN_ORBITAL_STATUS_SMOKE=PASS"

cat > "$SRC/$VERIFIED_MARKER" <<EOF
schema=1
project=ForgeClean
version=$VERSION
fmt=PASS
clippy=PASS
tests=PASS
release_build=PASS
gui_self_test=PASS
three_binary_gate=PASS
verified_at=$(date -Iseconds)
EOF
grep -Fxq "version=$VERSION" "$SRC/$VERIFIED_MARKER" || die "VERIFIED_SOURCE_MARKER_INVALID"
emit "VERIFIED_SOURCE_MARKER=PASS"

rm -f "$SOURCE_ZIP"
python3 - "$SRC" "$SOURCE_ZIP" <<'PY'
from pathlib import Path
import sys,zipfile
src=Path(sys.argv[1]); out=Path(sys.argv[2])
with zipfile.ZipFile(out,"w",compression=zipfile.ZIP_DEFLATED,compresslevel=9) as z:
    for p in sorted(src.rglob("*")):
        if not p.is_file():
            continue
        rel=p.relative_to(src)
        if any(part in {".git","target"} for part in rel.parts):
            continue
        z.write(p,Path(src.name)/rel)
PY
emit "SOURCE_ZIP=PASS"

flock -u 9
exec 9>&-
lock_open=0

AETHERFORGE_UPLOAD_ARTIFACTS=0 "$ORBIT" full >>"$VERIFY" 2>&1
emit "FULL_ORBIT_CANONICALIZATION=PASS"

systemctl --user disable --now "$LEGACY_AUTO_TIMER" >/dev/null 2>&1 || true
systemctl --user disable --now "$LEGACY_SWEEP_TIMER" >/dev/null 2>&1 || true
systemctl --user stop aetherforge-git-autosync.service >/dev/null 2>&1 || true
systemctl --user stop aetherforge-postreset-sweep.service >/dev/null 2>&1 || true

grep -P "^ForgeClean\t[0-9]+\t${HOME//\//\\/}/Downloads/ForgeClean-v1\.0\.10$" "$MAP" >/dev/null \
  || die "FORGECLEAN_SOURCE_MAP_NOT_1_0_10"
emit "FORGECLEAN_SOURCE_MAP_V1_0_10=PASS"

REMOTE_CARGO="$(gh api "repos/$REPO_FULL/contents/apps/ForgeClean/Cargo.toml?ref=main" --jq '.content' | base64 -d)"
grep -Eq '^version[[:space:]]*=[[:space:]]*"1\.0\.10"' <<<"$REMOTE_CARGO" \
  || die "REMOTE_FORGECLEAN_VERSION_NOT_1_0_10"

REMOTE_MARKER="$(gh api "repos/$REPO_FULL/contents/apps/ForgeClean/$VERIFIED_MARKER?ref=main" --jq '.content' | base64 -d)"
grep -Fxq "version=$VERSION" <<<"$REMOTE_MARKER" || die "REMOTE_VERIFIED_MARKER_MISSING"

REMOTE_SWEEP="$(gh api "repos/$REPO_FULL/contents/os/services/orbital-sync/aetherforge-postreset-sweep?ref=main" --jq '.content' | base64 -d)"
grep -Fq 'SOURCE_IMPORT_ATOMIC_SWAP=PASS' <<<"$REMOTE_SWEEP" || die "REMOTE_ATOMIC_IMPORT_MISSING"
grep -Fq 'SOURCE_IMPORT_SHRINK_GUARD=' <<<"$REMOTE_SWEEP" || die "REMOTE_SHRINK_GUARD_MISSING"
grep -Fq "$VERIFIED_MARKER" <<<"$REMOTE_SWEEP" || die "REMOTE_VERIFIED_AUTHORITY_MISSING"
emit "GITHUB_ATOMIC_SWEEP=PASS"

git -C "$REPO_DIR" fetch origin main --quiet
LOCAL_AFTER="$(git -C "$REPO_DIR" rev-parse HEAD)"
REMOTE_AFTER="$(git -C "$REPO_DIR" rev-parse origin/main)"
DIRTY_FINAL="$(git -C "$REPO_DIR" status --porcelain | wc -l | tr -d ' ')"
emit "LOCAL_HEAD_AFTER=$LOCAL_AFTER"
emit "REMOTE_HEAD_AFTER=$REMOTE_AFTER"
emit "DIRTY_FINAL=$DIRTY_FINAL"
[[ "$LOCAL_AFTER" == "$REMOTE_AFTER" ]] || die "REMOTE_PARITY_FAILED"
[[ "$DIRTY_FINAL" == "0" ]] || die "REPO_DIRTY_AFTER_FULL_ORBIT"
emit "REMOTE_PARITY=PASS"

mkdir -p "$HOME/.local/bin" "$STATE/bin-before"
for bin in forgeclean forgeclean-gui forgeclean-system; do
  if [[ -e "$HOME/.local/bin/$bin" ]]; then cp -a "$HOME/.local/bin/$bin" "$STATE/bin-before/$bin"; fi
  install -m 0755 "$SRC/target/release/$bin" "$HOME/.local/bin/$bin"
done
emit "BINARY_CUTOVER=PASS"

cat > "$ROLLBACK" <<EOF
#!/usr/bin/env bash
set -Eeuo pipefail
BACKUP='$STATE/bin-before'
for bin in forgeclean forgeclean-gui forgeclean-system; do
  if [[ -f "\$BACKUP/\$bin" ]]; then install -m 0755 "\$BACKUP/\$bin" "\$HOME/.local/bin/\$bin"; fi
done
echo 'FORGECLEAN_V1_0_10_ROLLBACK=PASS'
EOF
chmod +x "$ROLLBACK"
emit "ROLLBACK=PASS"

{
  sha256sum "$SOURCE_ZIP"
  sha256sum "$HOME/.local/bin/forgeclean"
  sha256sum "$HOME/.local/bin/forgeclean-gui"
  sha256sum "$HOME/.local/bin/forgeclean-system"
  sha256sum "$ROLLBACK"
} > "$SUMS"
emit "SHA256SUMS=PASS"

"$ORBIT" fast >>"$VERIFY" 2>&1
STATUS="$HOME/Downloads/AETHERFORGE-ORBITAL-SYNC-STATUS.txt"
grep -Fxq 'result=PASS' "$STATUS" || die "ORBITAL_STATUS_NOT_PASS"
grep -Fxq 'queue_ahead=0' "$STATUS" || die "ORBITAL_QUEUE_AHEAD"
grep -Fxq 'queue_behind=0' "$STATUS" || die "ORBITAL_QUEUE_BEHIND"
grep -Fxq 'dirty_count=0' "$STATUS" || die "ORBITAL_DIRTY"
grep -Fxq 'legacy_autosync_timer=disabled' "$STATUS" || die "LEGACY_AUTOSYNC_REACTIVATED"
grep -Fxq 'legacy_sweep_timer=disabled' "$STATUS" || die "LEGACY_SWEEP_REACTIVATED"
emit "ORBITAL_HEARTBEAT=PASS"

emit "FORGECLEAN_VERSION=$VERSION"
emit "AETHERFORGE_DELETION_ONLY_RECOVERY=PASS"
emit "AETHERFORGE_ATOMIC_SOURCE_IMPORT=PASS"
emit "AETHERFORGE_SOURCE_SHRINK_GUARD=PASS"
emit "FORGECLEAN_VERIFIED_SOURCE_AUTHORITY=PASS"
emit "FORGECLEAN_NETWORKCARD_MONITOR=LIVE_GUI_PAGE"
emit "FORGECLEAN_GITHUB_EXTERNAL_STORAGE=PASS"
emit "FORGECLEAN_SINGLE_SYNC_AUTHORITY=PASS"
emit "SOURCE_ZIP=$SOURCE_ZIP"
emit "SHA256SUMS=$SUMS"
emit "ROLLBACK_SCRIPT=$ROLLBACK"

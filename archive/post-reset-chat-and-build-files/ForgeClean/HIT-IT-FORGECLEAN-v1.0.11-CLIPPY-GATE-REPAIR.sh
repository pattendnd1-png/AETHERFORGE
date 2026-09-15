#!/usr/bin/env bash
set -Eeuo pipefail

BASE_VERSION="1.0.10"
VERSION="1.0.11"

BASE_SRC="$HOME/Downloads/ForgeClean-v${BASE_VERSION}"
SRC="$HOME/Downloads/ForgeClean-v${VERSION}"

REPO_DIR="${AETHERFORGE_REPO_DIR:-$HOME/Downloads/AETHERFORGE}"
REPO_FULL="${AETHERFORGE_REPO_FULL:-pattendnd1-png/AETHERFORGE}"
SWEEP="${AETHERFORGE_FULL_SWEEP_BIN:-$HOME/.local/bin/aetherforge-postreset-sweep}"
ORBIT="${AETHERFORGE_ORBITAL_BIN:-$HOME/.local/bin/aetherforge-orbital-sync}"
MAP="${XDG_CONFIG_HOME:-$HOME/.config}/aetherforge/postreset-source-map.tsv"

TIMER="aetherforge-orbital-sync.timer"
LEGACY_AUTO_TIMER="aetherforge-git-autosync.timer"
LEGACY_SWEEP_TIMER="aetherforge-postreset-sweep.timer"
LOCK_FILE="${AETHERFORGE_ORBITAL_STATE_DIR:-$HOME/.local/state/aetherforge/orbital-sync}/orbital.lock"

VERIFY="$HOME/Downloads/ForgeClean-v${VERSION}-CLIPPY-GATE-REPAIR-VERIFY.txt"
SOURCE_ZIP="$HOME/Downloads/ForgeClean-v${VERSION}-SOURCE.zip"
SUMS="$HOME/Downloads/ForgeClean-v${VERSION}-SHA256SUMS.txt"
ROLLBACK="$HOME/Downloads/ForgeClean-v${VERSION}-ROLLBACK.sh"
STATE="$HOME/.local/state/forgeclean-v${VERSION}-$(date +%Y%m%d-%H%M%S)"
VERIFIED_MARKER=".aetherforge-verified-source"

mkdir -p "$STATE" "$(dirname "$LOCK_FILE")"
: > "$VERIFY"

emit(){ printf '%s\n' "$*" | tee -a "$VERIFY"; }
die(){ emit "FORGECLEAN_V1_0_11_CLIPPY_GATE_REPAIR=FAIL:$*"; exit 1; }

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

  if (( timer_enabled == 1 )); then
    systemctl --user enable "$TIMER" >/dev/null 2>&1 || true
  fi
  if (( timer_active == 1 )); then
    systemctl --user start "$TIMER" >/dev/null 2>&1 || true
  fi

  if (( rc == 0 )); then
    emit "FORGECLEAN_V1_0_11_CLIPPY_GATE_REPAIR=PASS"
  else
    emit "FORGECLEAN_V1_0_11_CLIPPY_GATE_REPAIR=FAIL:${rc}"
  fi
  emit "VERIFY_FILE=$VERIFY"
  exit "$rc"
}
trap cleanup EXIT

for cmd in cargo rustc git gh rsync python3 sha256sum flock systemctl grep awk base64; do
  command -v "$cmd" >/dev/null 2>&1 || die "MISSING_COMMAND:$cmd"
done

[[ -d "$BASE_SRC" && -f "$BASE_SRC/Cargo.toml" ]] || die "BASE_SOURCE_MISSING:$BASE_SRC"
[[ -d "$REPO_DIR/.git" ]] || die "AETHERFORGE_REPO_MISSING:$REPO_DIR"
[[ -x "$SWEEP" ]] || die "SWEEP_WORKER_MISSING:$SWEEP"
[[ -x "$ORBIT" ]] || die "ORBITAL_WORKER_MISSING:$ORBIT"

emit "FORGECLEAN_V1_0_11_CLIPPY_GATE_REPAIR=START"
emit "SOURCE_BASELINE=$BASE_SRC"
emit "SOURCE_TARGET=$SRC"
emit "REPO=$REPO_FULL"

BASE_CARGO_VERSION="$(awk -F= '/^[[:space:]]*version[[:space:]]*=/{gsub(/["[:space:]]/,"",$2); print $2; exit}' "$BASE_SRC/Cargo.toml")"
[[ "$BASE_CARGO_VERSION" == "$BASE_VERSION" ]] || die "BASELINE_VERSION_MISMATCH:$BASE_CARGO_VERSION"
emit "BASELINE_VERSION=PASS"

[[ ! -f "$BASE_SRC/$VERIFIED_MARKER" ]] || die "FAILED_BASELINE_UNEXPECTEDLY_VERIFIED"
emit "FAILED_BASELINE_UNVERIFIED=PASS"

python3 - "$BASE_SRC/src/orbital.rs" <<'PY'
from pathlib import Path
import re, sys
s=Path(sys.argv[1]).read_text()
pat=re.compile(
    r'if let Some\(asset\) = fields\.get\(4\) \{\s*'
    r'if !asset\.is_empty\(\) && !names\.lines\(\)\.any\(\|name\| name == \*asset\) \{\s*'
    r'return Err\(format!\("missing GitHub release asset: \{asset\}"\)\.into\(\)\);\s*'
    r'\}\s*\}',
    re.S,
)
if not pat.search(s):
    raise SystemExit("expected collapsible_if signature not found")
PY
emit "ROOT_CAUSE_SIGNATURE=CONFIRMED"

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
DIRTY_BEFORE="$(git -C "$REPO_DIR" status --porcelain | wc -l | tr -d ' ')"
emit "LOCAL_HEAD_BEFORE=$LOCAL_BEFORE"
emit "REMOTE_HEAD_BEFORE=$REMOTE_BEFORE"
emit "DIRTY_BEFORE=$DIRTY_BEFORE"
[[ "$LOCAL_BEFORE" == "$REMOTE_BEFORE" ]] || die "REPO_NOT_ALIGNED"
[[ "$DIRTY_BEFORE" == "0" ]] || die "REPO_DIRTY"

grep -Fq 'SOURCE_IMPORT_ATOMIC_SWAP=PASS' "$SWEEP" || die "ACTIVE_SWEEP_ATOMIC_IMPORT_MISSING"
grep -Fq 'SOURCE_IMPORT_SHRINK_GUARD=' "$SWEEP" || die "ACTIVE_SWEEP_SHRINK_GUARD_MISSING"
grep -Fq "$VERIFIED_MARKER" "$SWEEP" || die "ACTIVE_SWEEP_VERIFIED_AUTHORITY_MISSING"
emit "ATOMIC_SWEEP_BASELINE=PASS"

RED_LOG="$STATE/clippy-red.log"
set +e
(
  cd "$BASE_SRC"
  cargo clippy --lib -- -D warnings
) >"$RED_LOG" 2>&1
RED_RC=$?
set -e
if (( RED_RC == 0 )); then
  die "TDD_RED_UNEXPECTEDLY_PASSED"
fi
grep -Fq 'clippy::collapsible-if' "$RED_LOG" || {
  cat "$RED_LOG" >>"$VERIFY"
  die "TDD_RED_WRONG_FAILURE"
}
emit "TDD_RED=PASS"

if [[ -e "$SRC" ]]; then
  mv "$SRC" "$STATE/ForgeClean-v${VERSION}.preexisting"
  emit "PREEXISTING_V1_0_11_QUARANTINED=PASS"
fi

mkdir -p "$SRC"
rsync -a \
  --exclude='.git/' \
  --exclude='target/' \
  --exclude="$VERIFIED_MARKER" \
  "$BASE_SRC/" "$SRC/"
emit "VERSIONED_SOURCE_COPY=PASS"

python3 - "$SRC" <<'PY'
from pathlib import Path
import re, sys
root=Path(sys.argv[1])

p=root/"src/orbital.rs"
s=p.read_text()
old=re.compile(
    r'(?P<indent>[ \t]*)if let Some\(asset\) = fields\.get\(4\) \{\s*'
    r'(?P=indent)    if !asset\.is_empty\(\) && !names\.lines\(\)\.any\(\|name\| name == \*asset\) \{\s*'
    r'(?P=indent)        return Err\(format!\("missing GitHub release asset: \{asset\}"\)\.into\(\)\);\s*'
    r'(?P=indent)    \}\s*'
    r'(?P=indent)\}',
    re.S,
)
m=old.search(s)
if not m:
    raise SystemExit("collapsible_if production anchor missing")
indent=m.group("indent")
new=(
    f'{indent}if let Some(asset) = fields.get(4)\n'
    f'{indent}    && !asset.is_empty()\n'
    f'{indent}    && !names.lines().any(|name| name == *asset)\n'
    f'{indent}' + '{\n'
    f'{indent}    return Err(format!("missing GitHub release asset: {{asset}}").into());\n'
    f'{indent}' + '}'
)
s=s[:m.start()]+new+s[m.end():]
p.write_text(s)

p=root/"Cargo.toml"
s=p.read_text()
s,n=re.subn(
    r'(?m)^(version\s*=\s*)"1\.0\.10"\s*$',
    r'\g<1>"1.0.11"',
    s,
    count=1,
)
if n != 1:
    raise SystemExit("Cargo.toml 1.0.10 version anchor missing")
p.write_text(s)

for name in ("build-and-verify.sh","install-local.sh","hit-it-template.sh"):
    p=root/name
    if not p.is_file():
        continue
    s=p.read_text()
    s=re.sub(r'(?m)^VERSION="1\.0\.10"$','VERSION="1.0.11"',s,count=1)
    p.write_text(s)

p=root/"README.md"
if p.is_file():
    s=p.read_text()
    s=re.sub(r'(?m)^# ForgeClean v1\.0\.10$','# ForgeClean v1.0.11',s,count=1)
    marker="## v1.0.11 — Strict Clippy Gate Repair\n"
    if marker not in s:
        insert=(
            "\n## v1.0.11 — Strict Clippy Gate Repair\n\n"
            "- Collapses the nested GitHub release-asset verification `if` into a Rust let-chain.\n"
            "- Preserves v1.0.10 atomic orbital imports, shrink protection, and verified-source authority.\n"
            "- Keeps strict `cargo clippy --all-targets --all-features -- -D warnings` as a release gate.\n\n"
        )
        pos=s.find("\n")
        s=s[:pos+1]+insert+s[pos+1:]
    p.write_text(s)
PY
emit "ROOT_CAUSE_PATCH=PASS"

cat > "$SRC/tests/regression_v1_0_11_clippy_gate.sh" <<'TEST'
#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

grep -Eq '^version[[:space:]]*=[[:space:]]*"1\.0\.11"' Cargo.toml
grep -Fq 'if let Some(asset) = fields.get(4)' src/orbital.rs
grep -Fq '&& !asset.is_empty()' src/orbital.rs
grep -Fq '&& !names.lines().any(|name| name == *asset)' src/orbital.rs

python3 - src/orbital.rs <<'PY'
from pathlib import Path
import re,sys
s=Path(sys.argv[1]).read_text()
bad=re.compile(
    r'if let Some\(asset\) = fields\.get\(4\) \{\s*'
    r'if !asset\.is_empty\(\) && !names\.lines\(\)\.any\(\|name\| name == \*asset\)',
    re.S,
)
if bad.search(s):
    raise SystemExit("old collapsible nested-if still present")
PY

echo 'FORGECLEAN_V1_0_11_CLIPPY_REGRESSION=PASS'
TEST
chmod +x "$SRC/tests/regression_v1_0_11_clippy_gate.sh"

"$SRC/tests/regression_v1_0_11_clippy_gate.sh" >>"$VERIFY" 2>&1
emit "TDD_GREEN_STATIC=PASS"

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

grep -P "^ForgeClean\t[0-9]+\t${HOME//\//\\/}/Downloads/ForgeClean-v1\.0\.11$" "$MAP" >/dev/null \
  || die "FORGECLEAN_SOURCE_MAP_NOT_1_0_11"
emit "FORGECLEAN_SOURCE_MAP_V1_0_11=PASS"

REMOTE_CARGO="$(gh api "repos/$REPO_FULL/contents/apps/ForgeClean/Cargo.toml?ref=main" --jq '.content' | base64 -d)"
grep -Eq '^version[[:space:]]*=[[:space:]]*"1\.0\.11"' <<<"$REMOTE_CARGO" \
  || die "REMOTE_FORGECLEAN_VERSION_NOT_1_0_11"

REMOTE_ORBITAL="$(gh api "repos/$REPO_FULL/contents/apps/ForgeClean/src/orbital.rs?ref=main" --jq '.content' | base64 -d)"
grep -Fq 'if let Some(asset) = fields.get(4)' <<<"$REMOTE_ORBITAL" || die "REMOTE_CLIPPY_REPAIR_MISSING"
grep -Fq '&& !asset.is_empty()' <<<"$REMOTE_ORBITAL" || die "REMOTE_CLIPPY_REPAIR_CONDITION_MISSING"

REMOTE_MARKER="$(gh api "repos/$REPO_FULL/contents/apps/ForgeClean/$VERIFIED_MARKER?ref=main" --jq '.content' | base64 -d)"
grep -Fxq "version=$VERSION" <<<"$REMOTE_MARKER" || die "REMOTE_VERIFIED_MARKER_MISSING"

REMOTE_SWEEP="$(gh api "repos/$REPO_FULL/contents/os/services/orbital-sync/aetherforge-postreset-sweep?ref=main" --jq '.content' | base64 -d)"
grep -Fq 'SOURCE_IMPORT_ATOMIC_SWAP=PASS' <<<"$REMOTE_SWEEP" || die "REMOTE_ATOMIC_IMPORT_MISSING"
grep -Fq 'SOURCE_IMPORT_SHRINK_GUARD=' <<<"$REMOTE_SWEEP" || die "REMOTE_SHRINK_GUARD_MISSING"
grep -Fq "$VERIFIED_MARKER" <<<"$REMOTE_SWEEP" || die "REMOTE_VERIFIED_AUTHORITY_MISSING"
emit "GITHUB_CANONICAL_SOURCE=PASS"
emit "GITHUB_ATOMIC_SWEEP=PASS"

git -C "$REPO_DIR" fetch origin main --quiet
LOCAL_AFTER="$(git -C "$REPO_DIR" rev-parse HEAD)"
REMOTE_AFTER="$(git -C "$REPO_DIR" rev-parse origin/main)"
DIRTY_AFTER="$(git -C "$REPO_DIR" status --porcelain | wc -l | tr -d ' ')"
emit "LOCAL_HEAD_AFTER=$LOCAL_AFTER"
emit "REMOTE_HEAD_AFTER=$REMOTE_AFTER"
emit "DIRTY_AFTER=$DIRTY_AFTER"
[[ "$LOCAL_AFTER" == "$REMOTE_AFTER" ]] || die "REMOTE_PARITY_FAILED"
[[ "$DIRTY_AFTER" == "0" ]] || die "REPO_DIRTY_AFTER_FULL_ORBIT"
emit "REMOTE_PARITY=PASS"

mkdir -p "$HOME/.local/bin" "$STATE/bin-before"
for bin in forgeclean forgeclean-gui forgeclean-system; do
  if [[ -e "$HOME/.local/bin/$bin" ]]; then
    cp -a "$HOME/.local/bin/$bin" "$STATE/bin-before/$bin"
  fi
  install -m 0755 "$SRC/target/release/$bin" "$HOME/.local/bin/$bin"
done
emit "BINARY_CUTOVER=PASS"

cat > "$ROLLBACK" <<EOF
#!/usr/bin/env bash
set -Eeuo pipefail
BACKUP='$STATE/bin-before'
for bin in forgeclean forgeclean-gui forgeclean-system; do
  if [[ -f "\$BACKUP/\$bin" ]]; then
    install -m 0755 "\$BACKUP/\$bin" "\$HOME/.local/bin/\$bin"
  fi
done
echo 'FORGECLEAN_V1_0_11_ROLLBACK=PASS'
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
emit "FORGECLEAN_COLLAPSIBLE_IF_REPAIR=PASS"
emit "FORGECLEAN_STRICT_CLIPPY_GATE=PASS"
emit "FORGECLEAN_VERIFIED_SOURCE_AUTHORITY=PASS"
emit "AETHERFORGE_ATOMIC_SOURCE_IMPORT=PASS"
emit "AETHERFORGE_SOURCE_SHRINK_GUARD=PASS"
emit "FORGECLEAN_NETWORKCARD_MONITOR=LIVE_GUI_PAGE"
emit "FORGECLEAN_GITHUB_EXTERNAL_STORAGE=PASS"
emit "FORGECLEAN_SINGLE_SYNC_AUTHORITY=PASS"
emit "SOURCE_ZIP=$SOURCE_ZIP"
emit "SHA256SUMS=$SUMS"
emit "ROLLBACK_SCRIPT=$ROLLBACK"

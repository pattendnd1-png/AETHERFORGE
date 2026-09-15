#!/usr/bin/env bash
set -Eeuo pipefail

OLD_VERSION="1.0.5"
VERSION="1.0.6"
OLD_SRC="$HOME/Downloads/ForgeClean-v${OLD_VERSION}"
SRC="$HOME/Downloads/ForgeClean-v${VERSION}"
REPO_DIR="${AETHERFORGE_REPO_DIR:-$HOME/Downloads/AETHERFORGE}"
REPO_FULL="${AETHERFORGE_REPO_FULL:-pattendnd1-png/AETHERFORGE}"
ORBIT="${AETHERFORGE_ORBITAL_BIN:-$HOME/.local/bin/aetherforge-orbital-sync}"
TIMER="aetherforge-orbital-sync.timer"
LEGACY_AUTO_TIMER="aetherforge-git-autosync.timer"
LEGACY_SWEEP_TIMER="aetherforge-postreset-sweep.timer"
LOCK_FILE="${AETHERFORGE_ORBITAL_STATE_DIR:-$HOME/.local/state/aetherforge/orbital-sync}/orbital.lock"

VERIFY="$HOME/Downloads/ForgeClean-v${VERSION}-LIB-SCOPE-REPAIR-VERIFY.txt"
SOURCE_ZIP="$HOME/Downloads/ForgeClean-v${VERSION}-SOURCE.zip"
SUMS="$HOME/Downloads/ForgeClean-v${VERSION}-SHA256SUMS.txt"
ROLLBACK="$HOME/Downloads/ForgeClean-v${VERSION}-ROLLBACK.sh"
STATE="$HOME/.local/state/forgeclean-v${VERSION}-$(date +%Y%m%d-%H%M%S)"

mkdir -p "$STATE" "$(dirname "$LOCK_FILE")"
: > "$VERIFY"

emit(){ printf '%s\n' "$*" | tee -a "$VERIFY"; }
die(){ emit "FORGECLEAN_V1_0_6_LIB_SCOPE_REPAIR=FAIL:$*"; exit 1; }

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
    emit "FORGECLEAN_V1_0_6_LIB_SCOPE_REPAIR=PASS"
  else
    emit "FORGECLEAN_V1_0_6_LIB_SCOPE_REPAIR=FAIL:${rc}"
  fi
  emit "VERIFY_FILE=$VERIFY"
  exit "$rc"
}
trap cleanup EXIT

for cmd in cargo rustc git gh rsync python3 sha256sum flock systemctl grep awk base64; do
  command -v "$cmd" >/dev/null 2>&1 || die "MISSING_COMMAND:$cmd"
done

[[ -d "$OLD_SRC" && -f "$OLD_SRC/Cargo.toml" ]] || die "FAILED_V1_0_5_SOURCE_MISSING:$OLD_SRC"
[[ -d "$REPO_DIR/.git" ]] || die "AETHERFORGE_REPO_MISSING:$REPO_DIR"
[[ -x "$ORBIT" ]] || die "ORBITAL_WORKER_MISSING:$ORBIT"

emit "FORGECLEAN_V1_0_6_LIB_SCOPE_REPAIR=START"
emit "SOURCE_BASELINE=$OLD_SRC"
emit "SOURCE_TARGET=$SRC"
emit "REPO=$REPO_FULL"

BASE_VERSION="$(awk -F= '/^[[:space:]]*version[[:space:]]*=/{gsub(/["[:space:]]/,"",$2); print $2; exit}' "$OLD_SRC/Cargo.toml")"
[[ "$BASE_VERSION" == "$OLD_VERSION" ]] || die "BASELINE_VERSION_MISMATCH:$BASE_VERSION"
emit "BASELINE_VERSION=PASS"

# Prove the exact failure signature is present in the failed source.
grep -Fq 'use forgeclean::orbital::' "$OLD_SRC/src/orbital_ui.rs" \
  || die "EXPECTED_ORBITAL_EXTERNAL_CRATE_IMPORT_NOT_FOUND"
grep -Fq 'use forgeclean::orbital_monitor_model::' "$OLD_SRC/src/orbital_ui.rs" \
  || die "EXPECTED_MONITOR_EXTERNAL_CRATE_IMPORT_NOT_FOUND"
emit "ROOT_CAUSE_SIGNATURE=CONFIRMED"

git -C "$REPO_DIR" fetch origin main --quiet
LOCAL_BEFORE="$(git -C "$REPO_DIR" rev-parse HEAD)"
REMOTE_BEFORE="$(git -C "$REPO_DIR" rev-parse origin/main)"
DIRTY_BEFORE="$(git -C "$REPO_DIR" status --porcelain | wc -l | tr -d ' ')"
emit "LOCAL_HEAD_BEFORE=$LOCAL_BEFORE"
emit "REMOTE_HEAD_BEFORE=$REMOTE_BEFORE"
emit "DIRTY_BEFORE=$DIRTY_BEFORE"
[[ "$LOCAL_BEFORE" == "$REMOTE_BEFORE" ]] || die "REPO_NOT_ALIGNED"
[[ "$DIRTY_BEFORE" == "0" ]] || die "REPO_DIRTY"

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

if [[ -e "$SRC" ]]; then
  mv "$SRC" "$STATE/ForgeClean-v${VERSION}.preexisting"
  emit "PREEXISTING_TARGET_BACKUP=$STATE/ForgeClean-v${VERSION}.preexisting"
fi

mkdir -p "$SRC"
rsync -a \
  --exclude='.git/' \
  --exclude='target/' \
  "$OLD_SRC/" "$SRC/"
emit "VERSIONED_SOURCE_COPY=PASS"

# TDD RED: the copied failed source must still violate the in-crate import rule.
cat > "$SRC/tests/regression_v1_0_6_lib_scope.sh" <<'TEST'
#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

! grep -Fq 'use forgeclean::' src/orbital_ui.rs
grep -Fq 'use crate::orbital::{local_storage_summary, read_status};' src/orbital_ui.rs
grep -Fq 'use crate::orbital_monitor_model::' src/orbital_ui.rs
grep -Fq 'pub mod orbital_ui;' src/lib.rs
grep -Fq 'Self::Orbital => "Orbital Sync"' src/gui.rs
grep -Fq 'Page::Orbital => crate::orbital_ui::show(ui)' src/gui.rs
grep -Eq '^version[[:space:]]*=[[:space:]]*"1\.0\.6"' Cargo.toml

echo 'FORGECLEAN_V1_0_6_LIB_SCOPE_REGRESSION=PASS'
TEST
chmod +x "$SRC/tests/regression_v1_0_6_lib_scope.sh"

if "$SRC/tests/regression_v1_0_6_lib_scope.sh" >>"$VERIFY" 2>&1; then
  die "TDD_RED_UNEXPECTEDLY_PASSED"
else
  emit "TDD_RED=PASS"
fi

# GREEN: one root-cause fix plus required semantic version increment.
python3 - "$SRC" <<'PY'
from pathlib import Path
import re, sys

root = Path(sys.argv[1])

p = root / "src/orbital_ui.rs"
s = p.read_text()

old1 = "use forgeclean::orbital::{local_storage_summary, read_status};"
new1 = "use crate::orbital::{local_storage_summary, read_status};"
old2 = "use forgeclean::orbital_monitor_model::{read_network_totals, read_sync_progress, NetworkTotals, SyncProgress};"
new2 = "use crate::orbital_monitor_model::{read_network_totals, read_sync_progress, NetworkTotals, SyncProgress};"

if old1 not in s or old2 not in s:
    raise SystemExit("expected v1.0.5 import anchors missing")
s = s.replace(old1, new1, 1)
s = s.replace(old2, new2, 1)
p.write_text(s)

p = root / "Cargo.toml"
s = p.read_text()
s2, n = re.subn(
    r'(?m)^(version\s*=\s*)"1\.0\.5"\s*$',
    r'\g<1>"1.0.6"',
    s,
    count=1,
)
if n != 1:
    raise SystemExit("Cargo.toml v1.0.5 version anchor missing")
p.write_text(s2)

for name in ("build-and-verify.sh", "install-local.sh", "hit-it-template.sh"):
    p = root / name
    if not p.is_file():
        continue
    s = p.read_text()
    s = re.sub(
        r'(?m)^VERSION="1\.0\.5"$',
        'VERSION="1.0.6"',
        s,
        count=1,
    )
    if name == "build-and-verify.sh":
        new_gate = 'run_gate FORGECLEAN_V1_0_6_LIB_SCOPE ./tests/regression_v1_0_6_lib_scope.sh || status=1\n'
        anchor = 'run_gate FORGECLEAN_V1_0_5_ORBITAL_UI ./tests/regression_v1_0_5_orbital_ui.sh || status=1\n'
        if new_gate not in s:
            if anchor not in s:
                raise SystemExit("v1.0.5 orbital regression gate anchor missing")
            s = s.replace(anchor, anchor + new_gate, 1)
    p.write_text(s)

p = root / "README.md"
s = p.read_text()
s = re.sub(r'(?m)^# ForgeClean v1\.0\.5$', '# ForgeClean v1.0.6', s, count=1)
marker = "## v1.0.6 — In-crate Orbital Monitor Repair\n"
if marker not in s:
    insert = (
        "\n## v1.0.6 — In-crate Orbital Monitor Repair\n\n"
        "- Repairs `orbital_ui` imports after the monitor became a library module.\n"
        "- Uses `crate::orbital` and `crate::orbital_monitor_model` from inside the ForgeClean library.\n"
        "- Preserves the NetworkCard-style Orbital Sync page introduced in v1.0.5.\n"
        "- No scheduler, storage, or migration architecture changes.\n\n"
    )
    first = s.find("\n")
    s = s[:first + 1] + insert + s[first + 1:]
p.write_text(s)
PY

emit "ROOT_CAUSE_PATCH=PASS"

"$SRC/tests/regression_v1_0_6_lib_scope.sh" >>"$VERIFY" 2>&1
emit "TDD_GREEN=PASS"

# Prove no library module still tries to import the library by its external crate name.
if grep -RIn --include='*.rs' '^use forgeclean::' "$SRC/src" >>"$VERIFY" 2>&1; then
  die "SELF_CRATE_IMPORT_STILL_PRESENT"
fi
emit "SELF_CRATE_IMPORT_SCAN=PASS"

for f in \
  "$SRC/build-and-verify.sh" \
  "$SRC/install-local.sh" \
  "$SRC/hit-it-template.sh" \
  "$SRC/tests/regression_v1_0_5_orbital_ui.sh" \
  "$SRC/tests/regression_v1_0_6_lib_scope.sh"
do
  [[ -f "$f" ]] || continue
  bash -n "$f"
done
emit "SHELL_SYNTAX=PASS"

cd "$SRC"

# Fresh compiler evidence before anything is installed.
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

# Package source before cutover.
rm -f "$SOURCE_ZIP"
python3 - "$SRC" "$SOURCE_ZIP" <<'PY'
from pathlib import Path
import sys, zipfile
src=Path(sys.argv[1]); out=Path(sys.argv[2])
with zipfile.ZipFile(out, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as z:
    for p in sorted(src.rglob("*")):
        if not p.is_file():
            continue
        rel=p.relative_to(src)
        if any(part in {".git","target"} for part in rel.parts):
            continue
        z.write(p, Path(src.name)/rel)
PY
emit "SOURCE_ZIP=PASS"

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
echo 'FORGECLEAN_V1_0_6_ROLLBACK=PASS'
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

# Release shared lock before the orbital worker takes it.
flock -u 9
exec 9>&-
lock_open=0

# v1.0.6 authority logic selects the highest valid ForgeClean-v* source.
AETHERFORGE_UPLOAD_ARTIFACTS=0 "$ORBIT" full >>"$VERIFY" 2>&1
emit "FULL_ORBIT_CANONICALIZATION=PASS"

systemctl --user disable --now "$LEGACY_AUTO_TIMER" >/dev/null 2>&1 || true
systemctl --user disable --now "$LEGACY_SWEEP_TIMER" >/dev/null 2>&1 || true
systemctl --user stop aetherforge-git-autosync.service >/dev/null 2>&1 || true
systemctl --user stop aetherforge-postreset-sweep.service >/dev/null 2>&1 || true

MAP="${XDG_CONFIG_HOME:-$HOME/.config}/aetherforge/postreset-source-map.tsv"
grep -P "^ForgeClean\t[0-9]+\t${HOME//\//\\/}/Downloads/ForgeClean-v1\.0\.6$" "$MAP" >/dev/null \
  || die "FORGECLEAN_SOURCE_MAP_NOT_1_0_6"
emit "FORGECLEAN_SOURCE_MAP_V1_0_6=PASS"

REMOTE_CARGO="$(gh api "repos/$REPO_FULL/contents/apps/ForgeClean/Cargo.toml?ref=main" --jq '.content' | base64 -d)"
grep -Eq '^version[[:space:]]*=[[:space:]]*"1\.0\.6"' <<<"$REMOTE_CARGO" \
  || die "REMOTE_FORGECLEAN_VERSION_NOT_1_0_6"

REMOTE_UI="$(gh api "repos/$REPO_FULL/contents/apps/ForgeClean/src/orbital_ui.rs?ref=main" --jq '.content' | base64 -d)"
grep -Fq 'use crate::orbital::{local_storage_summary, read_status};' <<<"$REMOTE_UI" \
  || die "REMOTE_CRATE_ORBITAL_IMPORT_MISSING"
grep -Fq 'use crate::orbital_monitor_model::' <<<"$REMOTE_UI" \
  || die "REMOTE_CRATE_MONITOR_IMPORT_MISSING"
if grep -Fq 'use forgeclean::' <<<"$REMOTE_UI"; then
  die "REMOTE_SELF_CRATE_IMPORT_STILL_PRESENT"
fi
grep -Fq 'pub fn show(ui: &mut egui::Ui)' <<<"$REMOTE_UI" \
  || die "REMOTE_LIVE_MONITOR_PAGE_MISSING"
emit "GITHUB_LIB_SCOPE_REPAIR=PASS"

REMOTE_GUI="$(gh api "repos/$REPO_FULL/contents/apps/ForgeClean/src/gui.rs?ref=main" --jq '.content' | base64 -d)"
grep -Fq 'Self::Orbital => "Orbital Sync"' <<<"$REMOTE_GUI" || die "REMOTE_ORBITAL_NAV_MISSING"
grep -Fq 'Page::Orbital => crate::orbital_ui::show(ui)' <<<"$REMOTE_GUI" || die "REMOTE_ORBITAL_RENDER_MISSING"
emit "GITHUB_ORBITAL_MONITOR_LIVE=PASS"

git -C "$REPO_DIR" fetch origin main --quiet
LOCAL_AFTER="$(git -C "$REPO_DIR" rev-parse HEAD)"
REMOTE_AFTER="$(git -C "$REPO_DIR" rev-parse origin/main)"
DIRTY_AFTER="$(git -C "$REPO_DIR" status --porcelain | wc -l | tr -d ' ')"
emit "LOCAL_HEAD_AFTER=$LOCAL_AFTER"
emit "REMOTE_HEAD_AFTER=$REMOTE_AFTER"
emit "DIRTY_AFTER=$DIRTY_AFTER"
[[ "$LOCAL_AFTER" == "$REMOTE_AFTER" ]] || die "REMOTE_PARITY_FAILED"
[[ "$DIRTY_AFTER" == "0" ]] || die "REPO_DIRTY_AFTER_CANONICALIZATION"
emit "REMOTE_PARITY=PASS"

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
emit "FORGECLEAN_LIB_SCOPE=PASS"
emit "FORGECLEAN_NETWORKCARD_MONITOR=LIVE_GUI_PAGE"
emit "FORGECLEAN_PIE_CHARTS=NONE"
emit "FORGECLEAN_GITHUB_EXTERNAL_STORAGE=PASS"
emit "FORGECLEAN_SINGLE_SYNC_AUTHORITY=PASS"
emit "SOURCE_ZIP=$SOURCE_ZIP"
emit "SHA256SUMS=$SUMS"
emit "ROLLBACK_SCRIPT=$ROLLBACK"

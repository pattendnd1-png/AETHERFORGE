#!/usr/bin/env bash
set -Eeuo pipefail

BASE_VERSION="1.0.5"
FAILED_VERSION="1.0.7"
VERSION="1.0.8"

BASE_SRC="$HOME/Downloads/ForgeClean-v${BASE_VERSION}"
FAILED_SRC="$HOME/Downloads/ForgeClean-v${FAILED_VERSION}"
SRC="$HOME/Downloads/ForgeClean-v${VERSION}"

REPO_DIR="${AETHERFORGE_REPO_DIR:-$HOME/Downloads/AETHERFORGE}"
REPO_FULL="${AETHERFORGE_REPO_FULL:-pattendnd1-png/AETHERFORGE}"
SWEEP="${AETHERFORGE_FULL_SWEEP_BIN:-$HOME/.local/bin/aetherforge-postreset-sweep}"
CANONICAL_SWEEP="$REPO_DIR/os/services/orbital-sync/aetherforge-postreset-sweep"
ORBIT="${AETHERFORGE_ORBITAL_BIN:-$HOME/.local/bin/aetherforge-orbital-sync}"
MAP="${XDG_CONFIG_HOME:-$HOME/.config}/aetherforge/postreset-source-map.tsv"

TIMER="aetherforge-orbital-sync.timer"
LEGACY_AUTO_TIMER="aetherforge-git-autosync.timer"
LEGACY_SWEEP_TIMER="aetherforge-postreset-sweep.timer"
LOCK_FILE="${AETHERFORGE_ORBITAL_STATE_DIR:-$HOME/.local/state/aetherforge/orbital-sync}/orbital.lock"

VERIFY="$HOME/Downloads/ForgeClean-v${VERSION}-LIBRARY-SCOPE-VERIFIED-AUTHORITY-VERIFY.txt"
SOURCE_ZIP="$HOME/Downloads/ForgeClean-v${VERSION}-SOURCE.zip"
SUMS="$HOME/Downloads/ForgeClean-v${VERSION}-SHA256SUMS.txt"
ROLLBACK="$HOME/Downloads/ForgeClean-v${VERSION}-ROLLBACK.sh"
STATE="$HOME/.local/state/forgeclean-v${VERSION}-$(date +%Y%m%d-%H%M%S)"
VERIFIED_MARKER=".aetherforge-verified-source"

mkdir -p "$STATE" "$(dirname "$LOCK_FILE")"
: > "$VERIFY"

emit(){ printf '%s\n' "$*" | tee -a "$VERIFY"; }
die(){ emit "FORGECLEAN_V1_0_8_LIBRARY_SCOPE_VERIFIED_AUTHORITY=FAIL:$*"; exit 1; }

timer_active=0
timer_enabled=0
lock_open=0

cleanup(){
  local rc=$?
  trap - EXIT
  if (( lock_open == 1 )); then flock -u 9 >/dev/null 2>&1 || true; exec 9>&- || true; fi
  systemctl --user disable --now "$LEGACY_AUTO_TIMER" >/dev/null 2>&1 || true
  systemctl --user disable --now "$LEGACY_SWEEP_TIMER" >/dev/null 2>&1 || true
  systemctl --user stop aetherforge-git-autosync.service >/dev/null 2>&1 || true
  systemctl --user stop aetherforge-postreset-sweep.service >/dev/null 2>&1 || true
  if (( timer_enabled == 1 )); then systemctl --user enable "$TIMER" >/dev/null 2>&1 || true; fi
  if (( timer_active == 1 )); then systemctl --user start "$TIMER" >/dev/null 2>&1 || true; fi
  if (( rc == 0 )); then
    emit "FORGECLEAN_V1_0_8_LIBRARY_SCOPE_VERIFIED_AUTHORITY=PASS"
  else
    emit "FORGECLEAN_V1_0_8_LIBRARY_SCOPE_VERIFIED_AUTHORITY=FAIL:${rc}"
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

emit "FORGECLEAN_V1_0_8_LIBRARY_SCOPE_VERIFIED_AUTHORITY=START"
emit "SOURCE_BASELINE=$BASE_SRC"
emit "SOURCE_TARGET=$SRC"
emit "REPO=$REPO_FULL"

BASE_CARGO_VERSION="$(awk -F= '/^[[:space:]]*version[[:space:]]*=/{gsub(/["[:space:]]/,"",$2); print $2; exit}' "$BASE_SRC/Cargo.toml")"
[[ "$BASE_CARGO_VERSION" == "$BASE_VERSION" ]] || die "BASELINE_VERSION_MISMATCH:$BASE_CARGO_VERSION"
emit "BASELINE_VERSION=PASS"

python3 - "$BASE_SRC/src/orbital_ui.rs" <<'PY'
from pathlib import Path
import re, sys
s = Path(sys.argv[1]).read_text()
a = re.search(r'(?m)^use\s+forgeclean::orbital::\{local_storage_summary,\s*read_status\};\s*$', s)
b = re.search(r'(?ms)^use\s+forgeclean::orbital_monitor_model::\{.*?\};\s*$', s)
if not (a and b):
    raise SystemExit("expected orbital_ui self-crate imports not found")
PY
emit "ROOT_CAUSE_SIGNATURE=CONFIRMED"

python3 - "$BASE_SRC" <<'PY'
from pathlib import Path
import sys
root = Path(sys.argv[1])
for p in [root/"src/main.rs", root/"src/gui_main.rs", root/"src/bin/forgeclean-system.rs"]:
    if not p.is_file() or "use forgeclean::" not in p.read_text():
        raise SystemExit(f"expected valid binary crate import missing: {p}")
PY
emit "BINARY_CRATE_IMPORT_PATTERN=CONFIRMED"

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

if [[ -e "$FAILED_SRC" ]]; then
  mv "$FAILED_SRC" "$STATE/ForgeClean-v${FAILED_VERSION}.failed"
  emit "FAILED_V1_0_7_QUARANTINED=PASS"
else
  emit "FAILED_V1_0_7_QUARANTINED=NOT_PRESENT"
fi

if [[ -e "$SRC" ]]; then
  mv "$SRC" "$STATE/ForgeClean-v${VERSION}.preexisting"
  emit "PREEXISTING_V1_0_8_QUARANTINED=PASS"
fi

mkdir -p "$SRC"
rsync -a --exclude='.git/' --exclude='target/' --exclude="$VERIFIED_MARKER" "$BASE_SRC/" "$SRC/"
emit "VERSIONED_SOURCE_COPY=PASS"

cat > "$SRC/tests/regression_v1_0_8_library_scope.sh" <<'TEST'
#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

python3 - . <<'PY'
from pathlib import Path
import sys
root = Path(sys.argv[1])
violations = []
for p in root.joinpath("src").rglob("*.rs"):
    rel = p.relative_to(root/"src")
    if rel in (Path("main.rs"), Path("gui_main.rs")) or (rel.parts and rel.parts[0] == "bin"):
        continue
    for i, line in enumerate(p.read_text(errors="ignore").splitlines(), 1):
        if line.lstrip().startswith("use forgeclean::"):
            violations.append(f"{rel}:{i}:{line}")
if violations:
    print("\n".join(violations))
    raise SystemExit(1)
PY

grep -Fq 'use crate::orbital::{local_storage_summary, read_status};' src/orbital_ui.rs
grep -Fq 'use crate::orbital_monitor_model::' src/orbital_ui.rs
grep -Fq 'pub mod orbital_ui;' src/lib.rs
grep -Fq 'Self::Orbital => "Orbital Sync"' src/gui.rs
grep -Fq 'Page::Orbital => crate::orbital_ui::show(ui)' src/gui.rs
grep -Eq '^version[[:space:]]*=[[:space:]]*"1\.0\.8"' Cargo.toml
grep -Fq 'use forgeclean::' src/main.rs
grep -Fq 'use forgeclean::' src/gui_main.rs
grep -Fq 'use forgeclean::' src/bin/forgeclean-system.rs

echo 'FORGECLEAN_V1_0_8_LIBRARY_SCOPE_REGRESSION=PASS'
TEST
chmod +x "$SRC/tests/regression_v1_0_8_library_scope.sh"

if "$SRC/tests/regression_v1_0_8_library_scope.sh" >>"$VERIFY" 2>&1; then
  die "TDD_RED_UNEXPECTEDLY_PASSED"
else
  emit "TDD_RED=PASS"
fi

python3 - "$SRC" <<'PY'
from pathlib import Path
import re, sys
root = Path(sys.argv[1])

p = root/"src/orbital_ui.rs"
s = p.read_text()
s, n1 = re.subn(
    r'(?m)^use\s+forgeclean::orbital::\{local_storage_summary,\s*read_status\};\s*$',
    'use crate::orbital::{local_storage_summary, read_status};',
    s, count=1,
)
s, n2 = re.subn(
    r'(?ms)^use\s+forgeclean::orbital_monitor_model::\{.*?\};\s*$',
    'use crate::orbital_monitor_model::{\n    NetworkTotals, SyncProgress, read_network_totals, read_sync_progress,\n};',
    s, count=1,
)
if n1 != 1 or n2 != 1:
    raise SystemExit(f"orbital_ui import replacements failed: orbital={n1} monitor={n2}")
p.write_text(s)

p = root/"Cargo.toml"
s = p.read_text()
s, n = re.subn(r'(?m)^(version\s*=\s*)"1\.0\.5"\s*$', r'\g<1>"1.0.8"', s, count=1)
if n != 1:
    raise SystemExit("Cargo.toml v1.0.5 version anchor missing")
p.write_text(s)

for name in ("build-and-verify.sh", "install-local.sh", "hit-it-template.sh"):
    p = root/name
    if not p.is_file():
        continue
    s = p.read_text()
    s = re.sub(r'(?m)^VERSION="1\.0\.5"$', 'VERSION="1.0.8"', s, count=1)
    if name == "build-and-verify.sh":
        gate = 'run_gate FORGECLEAN_V1_0_8_LIBRARY_SCOPE ./tests/regression_v1_0_8_library_scope.sh || status=1\n'
        anchor = 'run_gate FORGECLEAN_V1_0_5_ORBITAL_UI ./tests/regression_v1_0_5_orbital_ui.sh || status=1\n'
        if gate not in s:
            if anchor not in s:
                raise SystemExit("v1.0.5 orbital regression gate anchor missing")
            s = s.replace(anchor, anchor + gate, 1)
    p.write_text(s)

p = root/"README.md"
s = p.read_text()
s = re.sub(r'(?m)^# ForgeClean v1\.0\.5$', '# ForgeClean v1.0.8', s, count=1)
marker = "## v1.0.8 — Library-Scope + Verified Authority Repair\n"
if marker not in s:
    insert = (
        "\n## v1.0.8 — Library-Scope + Verified Authority Repair\n\n"
        "- Repairs `orbital_ui` self-crate imports with `crate::...` paths.\n"
        "- Scopes the self-import regression to library modules only; binary crates correctly keep `use forgeclean::...`.\n"
        "- Adds a verified-source marker only after fmt, strict Clippy, tests, release build, and GUI self-test pass.\n"
        "- Hardens orbital authority so failed ForgeClean trees cannot become canonical.\n\n"
    )
    pos = s.find("\n")
    s = s[:pos+1] + insert + s[pos+1:]
p.write_text(s)
PY
emit "ROOT_CAUSE_PATCH=PASS"

"$SRC/tests/regression_v1_0_8_library_scope.sh" >>"$VERIFY" 2>&1
emit "TDD_GREEN=PASS"

python3 - "$SRC" <<'PY'
from pathlib import Path
import sys
root = Path(sys.argv[1])
violations = []
for p in root.joinpath("src").rglob("*.rs"):
    rel = p.relative_to(root/"src")
    if rel in (Path("main.rs"), Path("gui_main.rs")) or (rel.parts and rel.parts[0] == "bin"):
        continue
    for i, line in enumerate(p.read_text(errors="ignore").splitlines(), 1):
        if line.lstrip().startswith("use forgeclean::"):
            violations.append(f"{rel}:{i}:{line}")
if violations:
    print("\n".join(violations))
    raise SystemExit(1)
PY
emit "LIBRARY_SELF_IMPORT_SCAN=PASS"

grep -Fq 'use forgeclean::' "$SRC/src/main.rs" || die "CLI_BINARY_IMPORT_PATTERN_BROKEN"
grep -Fq 'use forgeclean::' "$SRC/src/gui_main.rs" || die "GUI_BINARY_IMPORT_PATTERN_BROKEN"
grep -Fq 'use forgeclean::' "$SRC/src/bin/forgeclean-system.rs" || die "SYSTEM_BINARY_IMPORT_PATTERN_BROKEN"
emit "BINARY_CRATE_IMPORTS=PASS"

for f in "$SRC/build-and-verify.sh" "$SRC/install-local.sh" "$SRC/hit-it-template.sh" \
         "$SRC/tests/regression_v1_0_5_orbital_ui.sh" "$SRC/tests/regression_v1_0_8_library_scope.sh"; do
  [[ -f "$f" ]] || continue
  bash -n "$f"
done
emit "SHELL_SYNTAX=PASS"

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
grep -Fxq "version=$VERSION" "$SRC/$VERIFIED_MARKER" || die "VERIFIED_SOURCE_MARKER_VERSION_MISMATCH"
emit "VERIFIED_SOURCE_MARKER=PASS"

if grep -Fq "$VERIFIED_MARKER" "$SWEEP"; then
  emit "AUTHORITY_TDD_RED=ALREADY_HARDENED"
else
  emit "AUTHORITY_TDD_RED=PASS"
fi

cp -a "$SWEEP" "$STATE/aetherforge-postreset-sweep.before"

python3 - "$SWEEP" "$VERIFIED_MARKER" <<'PY'
from pathlib import Path
import sys
p = Path(sys.argv[1])
marker = sys.argv[2]
s = p.read_text()

start = s.find("ensure_forgeclean_source_mapping() {")
end = s.find("\ncapture_garuda_base() {", start)
if start < 0 or end < 0:
    raise SystemExit("ForgeClean authority block anchors missing")
head, block, tail = s[:start], s[start:end], s[end:]

if marker not in block:
    needle = (
        "    ver=tuple(map(int,m_ver.groups()))\n"
        "    candidates.append((ver, d.stat().st_mtime_ns, d))"
    )
    replacement = (
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
        raise SystemExit("ForgeClean candidate append anchor missing")
    block = block.replace(needle, replacement, 1)

old = '  elif [[ -f "$REPO_DIR/apps/ForgeClean/Cargo.toml" ]]; then'
new = f'  elif [[ -f "$REPO_DIR/apps/ForgeClean/Cargo.toml" && -f "$REPO_DIR/apps/ForgeClean/{marker}" ]]; then'
if old in block:
    block = block.replace(old, new, 1)
elif new not in block:
    raise SystemExit("ForgeClean fallback anchor missing")

p.write_text(head + block + tail)
PY

chmod 0755 "$SWEEP"
bash -n "$SWEEP"
grep -Fq "$VERIFIED_MARKER" "$SWEEP" || die "AUTHORITY_MARKER_PATCH_MISSING"
emit "AUTHORITY_TDD_GREEN=PASS"
emit "VERIFIED_SOURCE_AUTHORITY_PATCH=PASS"

mkdir -p "$(dirname "$CANONICAL_SWEEP")"
cp -f "$SWEEP" "$CANONICAL_SWEEP"
chmod 0755 "$CANONICAL_SWEEP"

git -C "$REPO_DIR" add -- os/services/orbital-sync/aetherforge-postreset-sweep
if ! git -C "$REPO_DIR" diff --cached --quiet; then
  git -C "$REPO_DIR" -c user.name='AETHERFORGE Automation' -c user.email='actions@users.noreply.github.com' \
    commit -m "Require verified ForgeClean source authority" >>"$VERIFY" 2>&1
  git -C "$REPO_DIR" push origin HEAD:main >>"$VERIFY" 2>&1
  emit "CANONICAL_AUTHORITY_PUSH=PASS"
else
  emit "CANONICAL_AUTHORITY_PUSH=ALREADY_CURRENT"
fi

rm -f "$SOURCE_ZIP"
python3 - "$SRC" "$SOURCE_ZIP" <<'PY'
from pathlib import Path
import sys, zipfile
src = Path(sys.argv[1]); out = Path(sys.argv[2])
with zipfile.ZipFile(out, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as z:
    for p in sorted(src.rglob("*")):
        if not p.is_file():
            continue
        rel = p.relative_to(src)
        if any(part in {".git", "target"} for part in rel.parts):
            continue
        z.write(p, Path(src.name)/rel)
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

grep -P "^ForgeClean\t[0-9]+\t${HOME//\//\\/}/Downloads/ForgeClean-v1\.0\.8$" "$MAP" >/dev/null \
  || die "FORGECLEAN_SOURCE_MAP_NOT_1_0_8"
emit "FORGECLEAN_SOURCE_MAP_V1_0_8=PASS"

REMOTE_CARGO="$(gh api "repos/$REPO_FULL/contents/apps/ForgeClean/Cargo.toml?ref=main" --jq '.content' | base64 -d)"
grep -Eq '^version[[:space:]]*=[[:space:]]*"1\.0\.8"' <<<"$REMOTE_CARGO" \
  || die "REMOTE_FORGECLEAN_VERSION_NOT_1_0_8"

REMOTE_MARKER="$(gh api "repos/$REPO_FULL/contents/apps/ForgeClean/$VERIFIED_MARKER?ref=main" --jq '.content' | base64 -d)"
grep -Fxq "version=$VERSION" <<<"$REMOTE_MARKER" || die "REMOTE_VERIFIED_MARKER_MISSING"

REMOTE_UI="$(gh api "repos/$REPO_FULL/contents/apps/ForgeClean/src/orbital_ui.rs?ref=main" --jq '.content' | base64 -d)"
grep -Fq 'use crate::orbital::{local_storage_summary, read_status};' <<<"$REMOTE_UI" || die "REMOTE_CRATE_ORBITAL_IMPORT_MISSING"
grep -Fq 'use crate::orbital_monitor_model::' <<<"$REMOTE_UI" || die "REMOTE_CRATE_MONITOR_IMPORT_MISSING"
if grep -Fq 'use forgeclean::' <<<"$REMOTE_UI"; then die "REMOTE_LIBRARY_SELF_IMPORT_STILL_PRESENT"; fi
emit "GITHUB_LIB_SCOPE_REPAIR=PASS"

REMOTE_GUI="$(gh api "repos/$REPO_FULL/contents/apps/ForgeClean/src/gui.rs?ref=main" --jq '.content' | base64 -d)"
grep -Fq 'Self::Orbital => "Orbital Sync"' <<<"$REMOTE_GUI" || die "REMOTE_ORBITAL_NAV_MISSING"
grep -Fq 'Page::Orbital => crate::orbital_ui::show(ui)' <<<"$REMOTE_GUI" || die "REMOTE_ORBITAL_RENDER_MISSING"
emit "GITHUB_ORBITAL_MONITOR_LIVE=PASS"

REMOTE_SWEEP="$(gh api "repos/$REPO_FULL/contents/os/services/orbital-sync/aetherforge-postreset-sweep?ref=main" --jq '.content' | base64 -d)"
grep -Fq "$VERIFIED_MARKER" <<<"$REMOTE_SWEEP" || die "REMOTE_AUTHORITY_MARKER_GUARD_MISSING"
emit "GITHUB_VERIFIED_AUTHORITY=PASS"

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
echo 'FORGECLEAN_V1_0_8_ROLLBACK=PASS'
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
emit "FORGECLEAN_LIBRARY_SCOPE_IMPORTS=PASS"
emit "FORGECLEAN_BINARY_CRATE_IMPORTS=PASS"
emit "FORGECLEAN_VERIFIED_SOURCE_AUTHORITY=PASS"
emit "FORGECLEAN_FAILED_SOURCE_AUTO_PROMOTION=BLOCKED"
emit "FORGECLEAN_NETWORKCARD_MONITOR=LIVE_GUI_PAGE"
emit "FORGECLEAN_PIE_CHARTS=NONE"
emit "FORGECLEAN_GITHUB_EXTERNAL_STORAGE=PASS"
emit "FORGECLEAN_SINGLE_SYNC_AUTHORITY=PASS"
emit "SOURCE_ZIP=$SOURCE_ZIP"
emit "SHA256SUMS=$SUMS"
emit "ROLLBACK_SCRIPT=$ROLLBACK"

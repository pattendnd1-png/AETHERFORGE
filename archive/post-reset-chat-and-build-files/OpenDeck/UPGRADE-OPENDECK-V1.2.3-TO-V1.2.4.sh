#!/usr/bin/env bash
set -euo pipefail

FROM_VERSION="1.2.3"
TO_VERSION="1.2.4"
FROM_TAG="v${FROM_VERSION}"
TO_TAG="v${TO_VERSION}"
ROOT_NAME="OpenDeck-Linux-${TO_TAG}"
EXPECTED_ZIP="972183143c955cd23e06adfdcacc0c9166acb03d152b53ed82a56582d643948b"
EXPECTED_TAR="ca5f7a227fc827499302192d08ad933916fbd6b764ad78523164f183e64c7c44"
HERE="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
APPLIER="$HERE/APPLY-V1.2.4.py"
INPUT=""
OUTPUT_DIR="${HOME}/Downloads"
INSTALL=1
KEEP_TREE=1
TEST_MODE="${OPENDECK_V124_TEST_MODE:-0}"

usage() {
  cat <<EOF
OpenDeck ${TO_TAG} — Windows workflow + DragonGlass + Twitch browser authorization

Usage: $(basename "$0") [options]
  --input PATH       Canonical OpenDeck ${FROM_TAG} ZIP/TAR donor
  --output-dir DIR   Output directory (default: ~/Downloads)
  --no-install       Build/package without installing locally
  --no-tree          Do not retain unpacked v1.2.4 source tree
  -h, --help         Show help
EOF
}
while (($#)); do
  case "$1" in
    --input) [[ $# -ge 2 ]] || exit 2; INPUT="$2"; shift 2 ;;
    --output-dir) [[ $# -ge 2 ]] || exit 2; OUTPUT_DIR="$2"; shift 2 ;;
    --no-install) INSTALL=0; shift ;;
    --no-tree) KEEP_TREE=0; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "ERROR: unknown argument: $1" >&2; usage >&2; exit 2 ;;
  esac
done

need() { command -v "$1" >/dev/null 2>&1 || { echo "ERROR: required command missing: $1" >&2; exit 3; }; }
for cmd in python3 sha256sum find grep sed tar; do need "$cmd"; done
[[ -x "$APPLIER" ]] || { echo "ERROR: missing v1.2.4 delta applier: $APPLIER" >&2; exit 4; }

find_canonical_donor() {
  local p sum
  while IFS= read -r -d '' p; do
    sum="$(sha256sum "$p" | awk '{print $1}')"
    case "$p:$sum" in
      *.zip:"$EXPECTED_ZIP") printf '%s\n' "$p"; return 0 ;;
      *.tar.gz:"$EXPECTED_TAR"|*.tgz:"$EXPECTED_TAR") printf '%s\n' "$p"; return 0 ;;
    esac
  done < <(find "$HOME/Downloads" -type f \( -name 'OpenDeck-Linux-v1.2.3.zip' -o -name 'OpenDeck-Linux-v1.2.3.tar.gz' -o -name 'OpenDeck-Linux-v1.2.3.tgz' \) -print0 2>/dev/null)
  return 1
}

if [[ -z "$INPUT" ]]; then INPUT="$(find_canonical_donor || true)"; fi
[[ -n "$INPUT" && -f "$INPUT" ]] || {
  echo "ERROR: exact canonical OpenDeck v1.2.3 ZIP/TAR donor not found under ~/Downloads" >&2
  echo "Expected ZIP SHA256=$EXPECTED_ZIP" >&2
  echo "Expected TAR SHA256=$EXPECTED_TAR" >&2
  exit 5
}
INPUT="$(cd "$(dirname "$INPUT")" && pwd -P)/$(basename "$INPUT")"
case "$INPUT" in
  *.zip) expected="$EXPECTED_ZIP" ;;
  *.tar.gz|*.tgz) expected="$EXPECTED_TAR" ;;
  *) echo "ERROR: v1.2.4 requires the canonical v1.2.3 ZIP/TAR archive" >&2; exit 6 ;;
esac
actual="$(sha256sum "$INPUT" | awk '{print $1}')"
[[ "$actual" == "$expected" ]] || { echo "ERROR: donor SHA256 mismatch: $actual" >&2; exit 7; }
echo "CANONICAL_V1_2_3_DONOR_SHA256=PASS:$actual"

mkdir -p "$OUTPUT_DIR"
OUTPUT_DIR="$(cd "$OUTPUT_DIR" && pwd -P)"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
EXTRACT="$TMP/extract"
STAGE="$TMP/stage"
DEST="$STAGE/$ROOT_NAME"
mkdir -p "$EXTRACT" "$DEST"

case "$INPUT" in
  *.zip) need unzip; unzip -q "$INPUT" -d "$EXTRACT" ;;
  *) tar -xzf "$INPUT" -C "$EXTRACT" ;;
esac

python3 - "$EXTRACT" "$DEST" <<'PY'
from pathlib import Path
import shutil, sys
src=Path(sys.argv[1]).resolve(); dest=Path(sys.argv[2]).resolve()
roots=[]
for cargo in src.rglob('Cargo.toml'):
    root=cargo.parent
    if (root/'apps/opendeck-studio/package.json').is_file() and (root/'crates').is_dir():
        roots.append(root)
unique=[]
for p in roots:
    if p not in unique: unique.append(p)
if len(unique)!=1:
    raise SystemExit(f'ERROR: expected exactly one OpenDeck source root, found {len(unique)}: {unique}')
root=unique[0]
for item in root.iterdir():
    target=dest/item.name
    if item.is_dir(): shutil.copytree(item,target,symlinks=True)
    else: shutil.copy2(item,target)
print(f'DONOR_ROOT={root}')
PY

# Canonical source-only staging: no donor build caches or patch debris.
rm -rf "$DEST/.git" "$DEST/target" "$DEST/apps/opendeck-studio/node_modules" \
       "$DEST/apps/opendeck-studio/dist" "$DEST/apps/opendeck-studio/src-tauri/target"
find "$DEST" -type f \( -name 'APPLY-OPENDECK-*' -o -name '*HOTFIX*' \) -delete 2>/dev/null || true

# Prove this is actually the v1.2.3 donor before mutation.
python3 - "$DEST" <<'PY'
import json,re,sys
from pathlib import Path
r=Path(sys.argv[1])
p=json.loads((r/'apps/opendeck-studio/package.json').read_text())
if p.get('version')!='1.2.3': raise SystemExit(f"ERROR: donor Studio version is {p.get('version')!r}, expected 1.2.3")
c=(r/'Cargo.toml').read_text(errors='replace')
if not re.search(r'(?m)^version\s*=\s*"1\.2\.3"\s*$',c): raise SystemExit('ERROR: root Cargo.toml is not v1.2.3')
print('V1_2_3_VERSION_CONTRACT=PASS')
PY

# Snapshot hardware/action implementation Rust. v1.2.4 is Studio/auth-launch only.
python3 - "$DEST" "$TMP/runtime-before.json" <<'PY'
from pathlib import Path
import hashlib,json,sys
root=Path(sys.argv[1]); out=Path(sys.argv[2])
prefixes=['crates/opendeck-device','crates/opendeck-actions','apps/opendeck-daemon']
d={}
for prefix in prefixes:
    base=root/prefix
    if not base.exists(): continue
    for p in base.rglob('*.rs'):
        d[str(p.relative_to(root))]=hashlib.sha256(p.read_bytes()).hexdigest()
out.write_text(json.dumps(d,sort_keys=True,indent=2)+'\n')
if not d: raise SystemExit('ERROR: no daemon/device/action Rust runtime files found for delta guard')
print(f'RUNTIME_DELTA_GUARD_SNAPSHOT=PASS:{len(d)}')
PY

# Version migration. Rename version-bearing scripts as a closure, then update content.
python3 - "$DEST" <<'PY'
from __future__ import annotations
from pathlib import Path
import json,sys
root=Path(sys.argv[1]); old='1.2.3'; new='1.2.4'
pairs=[
 (f'OpenDeck-Linux-v{old}',f'OpenDeck-Linux-v{new}'),
 (f'v{old}',f'v{new}'),(old,new),
 ('V1_2_3','V1_2_4'),('v1_2_3','v1_2_4'),('V123','V124'),('v123','v124')]
def replace(p:Path):
    if not p.is_file(): return
    try:s=p.read_text()
    except UnicodeDecodeError:return
    n=s
    for a,b in pairs:n=n.replace(a,b)
    if n!=s:p.write_text(n)
for p in root.rglob('Cargo.toml'): replace(p)
for rel in ['apps/opendeck-studio/package.json','apps/opendeck-studio/src-tauri/tauri.conf.json']:
    p=root/rel
    if p.is_file():
        d=json.loads(p.read_text()); d['version']=new; p.write_text(json.dumps(d,indent=2)+'\n')
replace(root/'packaging/arch/PKGBUILD')
for base in [root/'scripts', root/'docs'/'qualification']:
    if base.is_dir():
        files=sorted([p for p in base.rglob('*') if p.is_file() and any(x in p.name for x in [old,f'v{old}','1_2_3'])],key=lambda p:len(p.parts),reverse=True)
        for p in files:
            nn=p.name.replace(f'v{old}',f'v{new}').replace(old,new).replace('1_2_3','1_2_4')
            q=p.with_name(nn)
            if q==p: continue
            if q.exists() and q.read_bytes()!=p.read_bytes(): raise SystemExit(f'ERROR: conflicting version helper migration: {p} -> {q}')
            if q.exists(): p.unlink()
            else: p.rename(q)
for p in root.rglob('*'):
    if p.is_file() and ('scripts' in p.parts or ('docs' in p.parts and 'qualification' in p.parts) or p.name in {'START-HERE.txt','README.md','verify-release.sh','BUILD-ON-ARCH.sh'}): replace(p)
print('VERSION_MIGRATION=PASS:1.2.3->1.2.4')
PY

# TDD RED: install frontend dependencies, replace only the targeted test, and prove v1.2.3 production lacks the new opener behavior.
STUDIO="$DEST/apps/opendeck-studio"
if [[ "$TEST_MODE" != "1" ]]; then
  need node; need npm
  echo "=== TDD RED: Twitch browser authorization ==="
  if [[ -f "$STUDIO/package-lock.json" ]]; then npm --prefix "$STUDIO" ci --no-audit --no-fund; else npm --prefix "$STUDIO" install --no-audit --no-fund; fi
  cp "$STUDIO/src/components/TwitchAccountPanel.test.tsx" "$TMP/TwitchAccountPanel.test.original.tsx"
  cat > "$STUDIO/src/components/TwitchAccountPanel.test.tsx" <<'TSX'
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { TwitchAccountPanel } from './TwitchAccountPanel';
import { openTwitchAuthorization } from '../lib/twitchAuthorization';

vi.mock('../lib/twitchAuthorization', () => ({ openTwitchAuthorization: vi.fn(async () => undefined) }));
const openAuthorization = vi.mocked(openTwitchAuthorization);

describe('v1.2.4 Twitch browser authorization RED', () => {
  it('opens the Twitch verification URI after explicit sign in', async () => {
    const verification = 'https://www.twitch.tv/activate?public=true&device-code=ABCDEFGH';
    render(<TwitchAccountPanel identity={null} onSignIn={async () => ({ verification_uri: verification })} onSignOut={() => {}} />);
    fireEvent.click(screen.getByRole('button', { name: /sign in with twitch/i }));
    await waitFor(() => expect(openAuthorization).toHaveBeenCalledWith(verification));
  });
});
TSX
  set +e
  npm --prefix "$STUDIO" test -- --run src/components/TwitchAccountPanel.test.tsx >"$TMP/red.log" 2>&1
  red_rc=$?
  set -e
  if (( red_rc == 0 )); then
    cat "$TMP/red.log" >&2
    echo "ERROR: RED test unexpectedly passed before v1.2.4 implementation" >&2
    exit 20
  fi
  echo "TDD_RED_TWITCH_BROWSER_AUTH=PASS"
else
  echo "TDD_RED_TWITCH_BROWSER_AUTH=FIXTURE_MODE"
fi

# GREEN implementation.
python3 "$APPLIER" "$DEST"

# Delta guard: no daemon/device/action .rs implementation changes are allowed in this Studio-only release.
python3 - "$DEST" "$TMP/runtime-before.json" <<'PY'
from pathlib import Path
import hashlib,json,sys
root=Path(sys.argv[1]); before=json.loads(Path(sys.argv[2]).read_text()); after={}
for rel in before:
    p=root/rel
    if not p.is_file(): raise SystemExit(f'ERROR: runtime source disappeared: {rel}')
    after[rel]=hashlib.sha256(p.read_bytes()).hexdigest()
changed=[rel for rel in before if before[rel]!=after[rel]]
if changed:
    raise SystemExit('ERROR: v1.2.4 modified protected daemon/device/action runtime source: '+', '.join(changed))
print(f'RUNTIME_DELTA_GUARD=PASS:{len(before)}_RUST_FILES_UNCHANGED')
PY

# Static source gates always run, including package self-tests.
python3 "$DEST/scripts/verify/check-v1.2.4-windows-twitch.py"
CHECKER="$DEST/scripts/check-v1.2.4-source.sh"
[[ -x "$CHECKER" ]] || { echo "ERROR: canonical v1.2.4 source checker missing/not executable" >&2; exit 21; }
"$CHECKER"

VERIFY_OUT="$OUTPUT_DIR/OpenDeck-v1.2.4-HOST-VERIFY.txt"
: > "$VERIFY_OUT"
{
  echo 'OPENDECK_VERSION=1.2.4'
  echo 'BASELINE_VERSION=1.2.3'
  echo 'RELEASE_KIND=WINDOWS_WORKFLOW_DRAGONGLASS_TWITCH_BROWSER_AUTH'
  echo "CANONICAL_V1_2_3_DONOR_SHA256=$actual"
  echo 'RUNTIME_DELTA=STUDIO_ONLY'
  echo 'DAEMON_DEVICE_ACTION_RUST=UNCHANGED'
  echo 'WINDOWS_STREAMDECK_WORKFLOW_STYLE=PASS'
  echo 'DRAGONGLASS_DARK_THEME=PASS'
  echo 'TWITCH_EXPLICIT_BROWSER_AUTH_SOURCE=PASS'
} >> "$VERIFY_OUT"

if [[ "$TEST_MODE" != "1" ]]; then
  echo "=== TDD GREEN: targeted Twitch UI ==="
  npm --prefix "$STUDIO" test -- --run src/components/TwitchAccountPanel.test.tsx
  echo 'TDD_GREEN_TWITCH_BROWSER_AUTH=PASS' | tee -a "$VERIFY_OUT"

  echo "=== Frontend full tests ==="
  npm --prefix "$STUDIO" test -- --run
  echo 'FRONTEND_TESTS=PASS' | tee -a "$VERIFY_OUT"

  echo "=== Frontend production build ==="
  npm --prefix "$STUDIO" run build
  echo 'FRONTEND_BUILD=PASS' | tee -a "$VERIFY_OUT"

  need cargo
  echo "=== Rust fmt --check ==="
  (cd "$DEST" && cargo fmt --all -- --check)
  echo 'CARGO_FMT=PASS' | tee -a "$VERIFY_OUT"

  echo "=== Rust workspace tests ==="
  (cd "$DEST" && cargo test --workspace)
  echo 'CARGO_TEST=PASS' | tee -a "$VERIFY_OUT"

  echo "=== Rust Clippy: warnings are errors ==="
  (cd "$DEST" && cargo clippy --workspace --all-targets --no-deps -- -D warnings)
  echo 'CARGO_CLIPPY_D_WARNINGS=PASS' | tee -a "$VERIFY_OUT"

  echo "=== Rust release build ==="
  (cd "$DEST" && cargo build --workspace --release)
  echo 'CARGO_RELEASE_BUILD=PASS' | tee -a "$VERIFY_OUT"

  if [[ -x "$DEST/BUILD-ON-ARCH.sh" ]]; then
    echo "=== OpenDeck native Arch package gate ==="
    (cd "$DEST" && ./BUILD-ON-ARCH.sh)
    echo 'BUILD_ON_ARCH=PASS' | tee -a "$VERIFY_OUT"
  fi

  if (( INSTALL )); then
    echo "=== Local install ==="
    if [[ -x "$DEST/scripts/install-local.sh" ]]; then
      (cd "$DEST" && ./scripts/install-local.sh)
      echo 'INSTALL=PASS:PROJECT_INSTALL_LOCAL' | tee -a "$VERIFY_OUT"
    else
      echo 'INSTALL=SKIPPED:NO_PROJECT_INSTALLER' | tee -a "$VERIFY_OUT"
    fi
  else
    echo 'INSTALL=SKIPPED_BY_REQUEST' | tee -a "$VERIFY_OUT"
  fi
else
  echo 'HOST_GATES=SKIPPED:FIXTURE_MODE' >> "$VERIFY_OUT"
fi

# Canonical artifacts are emitted only after all required host gates pass.
ZIP_OUT="$OUTPUT_DIR/${ROOT_NAME}.zip"
TAR_OUT="$OUTPUT_DIR/${ROOT_NAME}.tar.gz"
SUM_OUT="$OUTPUT_DIR/${ROOT_NAME}-SHA256SUMS.txt"
rm -f "$ZIP_OUT" "$TAR_OUT" "$SUM_OUT"
need zip
(
  cd "$STAGE"
  zip -qr "$ZIP_OUT" "$ROOT_NAME"
  tar -czf "$TAR_OUT" "$ROOT_NAME"
)
(
  cd "$OUTPUT_DIR"
  sha256sum "$(basename "$ZIP_OUT")" "$(basename "$TAR_OUT")" > "$(basename "$SUM_OUT")"
  sha256sum -c "$(basename "$SUM_OUT")"
)

# Fresh archive structural gates.
for artifact in "$ZIP_OUT" "$TAR_OUT"; do
  fresh="$TMP/fresh-$(basename "$artifact" | tr '. ' '__')"
  mkdir -p "$fresh"
  case "$artifact" in *.zip) unzip -q "$artifact" -d "$fresh";; *) tar -xzf "$artifact" -C "$fresh";; esac
  froot="$fresh/$ROOT_NAME"
  [[ -f "$froot/Cargo.toml" && -f "$froot/apps/opendeck-studio/src/components/TwitchAccountPanel.tsx" ]] || {
    echo "ERROR: fresh artifact structurally incomplete: $artifact" >&2; exit 50;
  }
  python3 "$froot/scripts/verify/check-v1.2.4-windows-twitch.py"
  "$froot/scripts/check-v1.2.4-source.sh"
  echo "FRESH_ARTIFACT_SOURCE_GATE=PASS:$(basename "$artifact")"
done

if (( KEEP_TREE )); then
  FINAL_TREE="$OUTPUT_DIR/$ROOT_NAME"
  rm -rf "$FINAL_TREE"
  cp -a "$DEST" "$FINAL_TREE"
  echo "TREE=$FINAL_TREE" | tee -a "$VERIFY_OUT"
fi
{
  echo "ZIP=$ZIP_OUT"
  echo "TAR=$TAR_OUT"
  echo "SHA256=$SUM_OUT"
  echo 'TWITCH_RUNTIME_AUTHORIZATION=USER_INTERACTION_REQUIRED'
  echo 'OPENDECK_V1_2_4_BUILD_PIPELINE=PASS'
} | tee -a "$VERIFY_OUT"

echo "VERIFY=$VERIFY_OUT"
echo 'OPENDECK_V1_2_4_BUILD_PIPELINE=PASS'

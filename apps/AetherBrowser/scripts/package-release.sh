#!/usr/bin/env bash
set -euo pipefail

VERSION='2.1.60'
ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
OUT_DIR=${1:-"${AETHER_BROWSER_OUT_DIR:-$HOME/Downloads}"}
PREFIX="Aether-Browser-v${VERSION}"
ZIP="$OUT_DIR/${PREFIX}-source.zip"
INSTALL="$OUT_DIR/${PREFIX}-install.sh"
STATIC="$OUT_DIR/${PREFIX}-STATIC-VERIFY.txt"
PACKAGE="$OUT_DIR/${PREFIX}-PACKAGE-VERIFY.txt"
SHA="$OUT_DIR/${PREFIX}-SHA256SUMS.txt"
mkdir -p "$OUT_DIR"
STAGE_DIR=$(mktemp -d "$OUT_DIR/.${PREFIX}-package.XXXXXX")
trap 'rm -rf "$STAGE_DIR"' EXIT
ZIP_STAGE="$STAGE_DIR/$(basename "$ZIP")"
INSTALL_STAGE="$STAGE_DIR/$(basename "$INSTALL")"
STATIC_STAGE="$STAGE_DIR/$(basename "$STATIC")"
PACKAGE_STAGE="$STAGE_DIR/$(basename "$PACKAGE")"
SHA_STAGE="$STAGE_DIR/$(basename "$SHA")"

echo 'AETHER_BROWSER_RELEASE_STATIC_VERIFY=START'
set +e
(
  set -euo pipefail
  cd "$ROOT"
  cargo fmt --all -- --check
  mapfile -t CURRENT_CONTRACT_TESTS < <(
    find "$ROOT/tests" -maxdepth 1 -type f -name 'current-*.sh' -printf '%f\n' | sort
  )
  for t in "${CURRENT_CONTRACT_TESTS[@]}"; do
    bash "$ROOT/tests/$t"
  done
  echo 'AETHER_BROWSER_STATIC_VERIFY=PASS'
) 2>&1 | tee "$STATIC_STAGE"
static_rc=${PIPESTATUS[0]}
set -e
if (( static_rc != 0 )); then
  cp -f "$STATIC_STAGE" "$STATIC"
  echo "AETHER_BROWSER_RELEASE_STATIC_VERIFY=FAIL:$static_rc"
  exit "$static_rc"
fi
echo 'AETHER_BROWSER_RELEASE_STATIC_VERIFY=PASS'

python3 - "$ROOT" "$ZIP_STAGE" "$PREFIX" <<'PY'
from pathlib import Path
import sys,zipfile
root=Path(sys.argv[1]).resolve(); out=Path(sys.argv[2]).resolve(); prefix=sys.argv[3]
exclude={'.git','.worktrees','target','.aether-tools','__pycache__'}
with zipfile.ZipFile(out,'w',compression=zipfile.ZIP_DEFLATED,compresslevel=9) as zf:
    for path in sorted(root.rglob('*')):
        rel=path.relative_to(root)
        if any(part in exclude for part in rel.parts): continue
        if path.is_file():
            info=zipfile.ZipInfo(f'{prefix}/{rel.as_posix()}')
            info.date_time=(2026,9,9,0,0,0)
            mode=path.stat().st_mode & 0o777
            info.external_attr=(mode & 0xFFFF)<<16
            info.compress_type=zipfile.ZIP_DEFLATED
            zf.writestr(info,path.read_bytes(),compress_type=zipfile.ZIP_DEFLATED,compresslevel=9)
PY

cp "$ROOT/scripts/install-host.sh" "$INSTALL_STAGE"
chmod +x "$INSTALL_STAGE"

python3 - "$ZIP_STAGE" "$PREFIX" <<'PY' > "$PACKAGE_STAGE"
import sys,zipfile,re
z=zipfile.ZipFile(sys.argv[1]); prefix=sys.argv[2]+'/'
names=z.namelist()
required=[
'Cargo.toml',
'README.md',
'CHANGELOG.md',
'crates/aether-browser/src/main.rs',
'crates/aether-browser/src/lib.rs',
'crates/aether-engine-servo/src/live.rs',
'crates/aether-compat/src/lib.rs',
'crates/aether-ui/src/lib.rs',
'crates/aether-native-pages/src/lib.rs',
'crates/aether-creator-integrations/src/vendor_packages.rs',
'crates/aether-stream-studio/src/obs.rs',
'crates/aether-stream-studio/src/bin/aether-obs-verify.rs',
'crates/aether-ui/assets/aetherforge-cosmic-wallpaper.jpg',
'crates/aether-ui/assets/aether-stream-studio-preview.png',
'crates/aether-ui/assets/aether-vault-preview.png',
'scripts/verify.sh',
'scripts/install-host.sh',
'scripts/install-current-tree.sh',
'scripts/obs-runtime-test.sh',
'scripts/streamlabs-runtime-test.sh',
'scripts/velora-runtime-test.sh',
'scripts/vendor-package-normalize.py',
'scripts/browser-takeover.sh',
'scripts/browser-takeover-rollback.sh',
'packaging/systemd/aether-browser-media.service',
'packaging/desktop/org.aetherforge.AetherBrowser.desktop.in',
'tests/current-clean-break.sh',
'tests/current-release-identity.sh',
'tests/current-package-contract.sh',
'tests/current-install-contract.sh',
'tests/current-v2-1-48-authoritative-vendor-packages.sh',
'tests/current-v2-1-49-visible-test-readiness.sh',
'tests/current-v2-1-49-gx-dragon-glass-shell.sh',
'tests/current-v2-1-49-browser-takeover.sh',
'tests/current-v2-1-50-controller-software-removal.sh',
'tests/current-v2-1-50-opera-gx-deb-refit.sh',
'tests/current-v2-1-51-stale-browser-shutdown.sh',
'tests/current-v2-1-52-gx-rust-1-98-clippy.sh',
'tests/current-v2-1-53-no-removed-controller-install-hooks.sh',
'tests/current-v2-1-54-loop-break-closure.sh',
'tests/current-v2-1-56-assistant-removal.sh',
'tests/current-v2-1-58-release-package-observability.sh',
'tests/current-v2-1-59-rustfmt-closure.sh',
'tests/current-v2-1-60-dragonglass-policy.sh',
'crates/aether-creator-integrations/tests/vendor_packages.rs',
]
missing=[x for x in required if prefix+x not in names]
if missing:
    print('AETHER_BROWSER_PACKAGE_VERIFY=FAIL:missing='+','.join(missing)); raise SystemExit(1)
for forbidden in ('/.git/','/.worktrees/','/target/','/.aether-tools/'):
    if any(forbidden in '/'+n for n in names):
        print('AETHER_BROWSER_PACKAGE_VERIFY=FAIL:forbidden='+forbidden); raise SystemExit(1)
removed_tokens=(
    'crates/aether-deck/', 'streamdeck-plus-runtime-test.sh', 'elgato-streamdeck-pkg-normalize.py',
    'aether-browser-deck.service', '70-aetherforge-streamdeck.rules', 'org.aetherforge.StreamDeckStudio.desktop',
    'aether-deck-daemon', 'aether-deck-monitor', 'aether-streamdeck-studio',
)
for token in removed_tokens:
    if any(token.lower() in n.lower() for n in names):
        print('AETHER_BROWSER_PACKAGE_VERIFY=FAIL:removed-streamdeck='+token); raise SystemExit(1)
for n in names:
    if n.lower().endswith(('.exe','.msi','.pkg','.deb')):
        print('AETHER_BROWSER_PACKAGE_VERIFY=FAIL:vendor-installer='+n); raise SystemExit(1)
# Every top-level shell test in the source archive must be part of the current unversioned suite.
for n in names:
    if n.startswith(prefix+'tests/') and n.endswith('.sh') and '/current-' not in '/'+n:
        print('AETHER_BROWSER_PACKAGE_VERIFY=FAIL:historical-test='+n); raise SystemExit(1)
print(f'AETHER_BROWSER_PACKAGE_FILES={len(names)}')
print('AETHER_BROWSER_PACKAGE_CLEAN_BREAK=PASS')
print('AETHER_BROWSER_PACKAGE_NATIVE_CHROME=PASS')
print('AETHER_BROWSER_PACKAGE_ASSISTANT_REMOVAL=PASS')
print('AETHER_BROWSER_PACKAGE_VERIFY=PASS')
PY

cd "$STAGE_DIR"
sha256sum "$(basename "$ZIP_STAGE")" "$(basename "$INSTALL_STAGE")" "$(basename "$STATIC_STAGE")" "$(basename "$PACKAGE_STAGE")" > "$SHA_STAGE"

# Promote only after every package gate has passed. Existing handoff artifacts remain intact on failure.
mv -f "$ZIP_STAGE" "$ZIP"
mv -f "$INSTALL_STAGE" "$INSTALL"
mv -f "$STATIC_STAGE" "$STATIC"
mv -f "$PACKAGE_STAGE" "$PACKAGE"
mv -f "$SHA_STAGE" "$SHA"
trap - EXIT
rm -rf "$STAGE_DIR"
echo 'AETHER_BROWSER_RELEASE_PACKAGE=PASS'
echo "AETHER_BROWSER_RELEASE_DIR=$OUT_DIR"

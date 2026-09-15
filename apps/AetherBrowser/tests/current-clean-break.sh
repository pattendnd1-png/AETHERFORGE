#!/usr/bin/env bash
set -euo pipefail
python3 - <<'PY'
from pathlib import Path
import re
root=Path('.').resolve()
allowed_scripts={
'browser-takeover-rollback.sh',
'browser-takeover.sh',
'build-first-party-release.sh',
'classify-pacman-qkk.sh',
'create-root-owned-payload-tar.sh',
'host-build.sh',
'install-current-tree.sh',
'install-host.sh',
'invalidate-first-party-cache.sh',
'media-runtime-test.sh',
'obs-runtime-test.sh',
'package-consolidated.sh',
'package-release.sh',
'prepare-media-safe-renderer.sh',
'repair-private-install-dirs.sh',
'rollback-host.sh',
'stability-acceptance.sh',
'streamlabs-runtime-test.sh',
'uninstall-host.sh',
'velora-runtime-test.sh',
'vendor-package-normalize.py',
'verify.sh',
'version-smoke.sh',
}
actual_tests={p.name for p in (root/'tests').glob('*') if p.is_file()}
actual_scripts={p.name for p in (root/'scripts').glob('*') if p.is_file()}
non_current_tests={name for name in actual_tests if not (name.startswith('current-') and name.endswith('.sh'))}
if non_current_tests:
    raise SystemExit('AETHER_BROWSER_CLEAN_BREAK=FAIL:non-current-tests:'+','.join(sorted(non_current_tests)))
if not actual_tests:
    raise SystemExit('AETHER_BROWSER_CLEAN_BREAK=FAIL:no-current-tests')
if actual_scripts != allowed_scripts:
    raise SystemExit('AETHER_BROWSER_CLEAN_BREAK=FAIL:script-set:'+','.join(sorted(actual_scripts ^ allowed_scripts)))
# Inspect production/control files only; the guard itself is excluded.
files=[]
for top in ('crates','scripts','packaging','integration'):
    for p in (root/top).rglob('*'):
        if p.is_file() and 'target' not in p.parts and '.aether-tools' not in p.parts and p.name!='Cargo.lock':
            try: files.append((p,p.read_text()))
            except UnicodeDecodeError: pass
browser_owned = [
    root/'crates/aether-ui/src/lib.rs',
    root/'crates/aether-engine-servo/src/live.rs',
    root/'crates/aether-browser/src/main.rs',
    root/'crates/aether-native-pages/src/lib.rs',
]
for p in browser_owned:
    text=p.read_text()
    if re.search(r'\bpanel\b|PANEL_|_PANEL|PanelMode|panel[_ -]state|dragon.?glass.?panel|shell_capture|FIXED_', text, re.I):
        raise SystemExit(f'AETHER_BROWSER_CLEAN_BREAK=FAIL:panel-language:{p.relative_to(root)}')

for p,s in files:
    if re.search(r'--m\d+-status|M\d+FeatureState|AETHER_BROWSER_MILESTONE=M\d+|STATUS_COMPAT',s):
        raise SystemExit(f'AETHER_BROWSER_CLEAN_BREAK=FAIL:milestone-compat:{p.relative_to(root)}')
    if re.search(r'PREVIOUS_(TARGET|HELPER)_|reused-v\d',s):
        raise SystemExit(f'AETHER_BROWSER_CLEAN_BREAK=FAIL:prior-build-reuse:{p.relative_to(root)}')
    if re.search(r'UNIFIED_BROWSER_SURFACE|UNIFIED_UI\|ACTIVE_WEB_CONTENT|RUNNING_NONCOMPOSITED|ChromeDelegate|chrome_webview|chrome_context|render_patch_script|render_document', s):
        raise SystemExit(f'AETHER_BROWSER_CLEAN_BREAK=FAIL:retired-render-architecture:{p.relative_to(root)}')
    # Any browser release reference in production/control must be the current release.
    for m in re.finditer(r'Aether-Browser-v(\d+\.\d+\.\d+)',s):
        if m.group(1) != '2.1.60':
            raise SystemExit(f'AETHER_BROWSER_CLEAN_BREAK=FAIL:prior-release-ref:{p.relative_to(root)}:{m.group(1)}')
    for m in re.finditer(r'V(\d+)_(\d+)_(\d+)', s):
        if '.'.join(m.groups()) != '2.1.60':
            raise SystemExit(f'AETHER_BROWSER_CLEAN_BREAK=FAIL:prior-version-marker:{p.relative_to(root)}:{m.group(0)}')
print('AETHER_BROWSER_CLEAN_BREAK=PASS')
PY

#!/usr/bin/env bash
set -euo pipefail
UI=crates/aether-ui/src/lib.rs
fail(){ echo "AETHER_BROWSER_CURRENT_RENDER=FAIL:$1"; exit 1; }
[[ ! -e crates/aether-ui/assets/browser.html ]] || fail browser-html-present
python3 - <<'PY'
from pathlib import Path
s=Path('crates/aether-ui/src/lib.rs').read_text().lower()
required=[
'aether browser','built for those who play, create, and explore.','browse freely','stream instantly','keep it yours','go further',
'aether stream studio','aether vault','media hub','stream studio','creator hub','downloads','stream status',
'more than a browser','speed','privacy','creators','without limits'
]
for token in required:
    if token not in s:
        raise SystemExit('AETHER_BROWSER_CURRENT_RENDER=FAIL:missing:'+token)
for token in ['paint_home','paint_top_chrome','paint_left_launcher','paint_right_utility','paint_status','aetherforge-cosmic-wallpaper.jpg','aether-stream-studio-preview.png','aether-vault-preview.png']:
    if token.lower() not in s:
        raise SystemExit('AETHER_BROWSER_CURRENT_RENDER=FAIL:missing-native:'+token)
if 'if model.native_surface_owns_content()' not in Path('crates/aether-ui/src/lib.rs').read_text():
    raise SystemExit('AETHER_BROWSER_CURRENT_RENDER=FAIL:web-content-overpaint-policy-missing')
print('AETHER_BROWSER_CURRENT_RENDER=PASS')
PY

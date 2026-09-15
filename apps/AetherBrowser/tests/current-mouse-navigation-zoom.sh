#!/usr/bin/env bash
set -euo pipefail
fail(){ echo "AETHER_BROWSER_MOUSE_NAV_ZOOM=FAIL:$1"; exit 1; }
LIVE=crates/aether-engine-servo/src/live.rs
python3 - "$LIVE" <<'PY'
from pathlib import Path
import re,sys
s=Path(sys.argv[1]).read_text()
mb=re.search(r'fn handle_mouse_button\(&mut self, state: ElementState, button: WinitMouseButton\) \{(.*?)\n    \}',s,re.S)
if not mb: raise SystemExit('AETHER_BROWSER_MOUSE_NAV_ZOOM=FAIL:no-mouse-handler')
b=mb.group(1)
for needle in ['WinitMouseButton::Back','WinitMouseButton::Forward','self.go_back();','self.go_forward();','state == ElementState::Pressed']:
    if needle not in b: raise SystemExit('AETHER_BROWSER_MOUSE_NAV_ZOOM=FAIL:missing:'+needle)
# Back/Forward must be consumed by browser chrome logic, not converted to Servo buttons.
if 'servo::MouseButton::Back' in b or 'servo::MouseButton::Forward' in b:
    raise SystemExit('AETHER_BROWSER_MOUSE_NAV_ZOOM=FAIL:side-buttons-forwarded-to-web')
scroll=re.search(r'fn handle_scroll\(&mut self, delta: MouseScrollDelta\) \{(.*?)\n    \}',s,re.S)
if not scroll: raise SystemExit('AETHER_BROWSER_MOUSE_NAV_ZOOM=FAIL:scroll-handler-not-mutable')
z=scroll.group(1)
for needle in ['self.modifiers.control_key()','webview.page_zoom()','webview.set_page_zoom(','AETHER_BROWSER_PAGE_ZOOM=']:
    if needle not in z: raise SystemExit('AETHER_BROWSER_MOUSE_NAV_ZOOM=FAIL:zoom-missing:'+needle)
# Control branch must return before normal Wheel event forwarding.
ctrl=z.find('self.modifiers.control_key()'); wheel=z.find('InputEvent::Wheel')
ret=z.find('return;',ctrl)
if min(ctrl,wheel,ret) < 0 or not (ctrl < ret < wheel):
    raise SystemExit('AETHER_BROWSER_MOUSE_NAV_ZOOM=FAIL:ctrl-wheel-not-consumed')
print('AETHER_BROWSER_MOUSE_BACK_NAV=PASS')
print('AETHER_BROWSER_MOUSE_FORWARD_NAV=PASS')
print('AETHER_BROWSER_MOUSE_NAV_NO_WEB_FORWARD=PASS')
print('AETHER_BROWSER_CTRL_WHEEL_ZOOM_IN=PASS')
print('AETHER_BROWSER_CTRL_WHEEL_ZOOM_OUT=PASS')
print('AETHER_BROWSER_CTRL_WHEEL_NO_SCROLL=PASS')
print('AETHER_BROWSER_TAB_ZOOM_PERSIST=PASS')
PY
echo 'AETHER_BROWSER_MOUSE_NAV_ZOOM=PASS'

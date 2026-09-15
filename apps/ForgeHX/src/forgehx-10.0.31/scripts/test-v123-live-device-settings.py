#!/usr/bin/env python3
from pathlib import Path

root = Path(__file__).resolve().parents[1]
app = (root/'crates/forgehx-gui/src/app.rs').read_text()
page = (root/'crates/forgehx-gui/src/device_page.rs').read_text()
mouse = (root/'crates/forgehx-gui/src/mouse.rs').read_text()
lighting = (root/'crates/forgehx-gui/src/lighting.rs').read_text()
core = (root/'crates/forgehx-core/src/lib.rs').read_text()
haste = (root/'crates/forgehx-mouse/src/protocol/haste_v1.rs').read_text()

checks = {
    'pane selection refreshes selected hardware state':
        'pane_changed' in page and 'sync_selected_backend_state' in app and 'selected_tab_before' in app,
    'device writes use a debounced live-write queue':
        'pending_device_write' in app and 'queue_device_action' in app and 'commit_device_action' in app,
    'successful and failed device writes reconcile from hardware':
        'reconcile_device_after_write' in app and 'sync_selected_backend_state' in app,
    'lighting edits are live with no Apply button':
        'Apply lighting' not in lighting and 'let before = config.clone();' in lighting and 'config != &before' in lighting,
    'mouse DPI/polling/lift/profile controls are live':
        'Apply DPI' not in mouse and 'Button::new("Apply")' not in mouse and 'Button::new("Activate")' not in mouse,
    'button assignment no longer needs an Apply button':
        'Apply assignment' not in mouse,
    'mouse hardware state carries lighting readback':
        'pub lighting: Option<LightingConfig>' in core,
    'Haste state requests LED readback':
        'REPORT_LED_SETTINGS' in haste and 'parse_lighting_report' in haste and 'direct_led_settings_packet' in haste and 'lighting:' in haste,
}
failed = [name for name, ok in checks.items() if not ok]
for name, ok in checks.items():
    print(('PASS' if ok else 'FAIL') + ': ' + name)
if failed:
    raise SystemExit('ForgeHX 10.0.6 live device settings contract failed: ' + ', '.join(failed))
print('PASS: ForgeHX 10.0.6 live device settings contract')

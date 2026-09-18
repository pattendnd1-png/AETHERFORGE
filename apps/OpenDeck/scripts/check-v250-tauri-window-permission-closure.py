#!/usr/bin/env python3
from pathlib import Path
import json, sys
root = Path(__file__).resolve().parents[1]
cap = json.loads((root/'apps/opendeck-studio/src-tauri/capabilities/default.json').read_text())
permissions = set(cap.get('permissions', []))
required = {
    'core:window:allow-show',
    'core:window:allow-maximize',
    'core:window:allow-minimize',
    'core:window:allow-toggle-maximize',
    'core:window:allow-close',
    'core:window:allow-start-dragging',
    'core:window:allow-start-resize-dragging',
}
missing = sorted(required - permissions)
lib = (root/'apps/opendeck-studio/src-tauri/src/lib.rs').read_text()
main = (root/'apps/opendeck-studio/src/main.tsx').read_text()
menu = (root/'scripts/check-v250-system-menu-launch.sh').read_text()
errors = []
if missing: errors.append('missing_permissions=' + ','.join(missing))
if 'startup::prepare_main_window' not in lib: errors.append('native_startup_window_show_missing')
if 'startupVisibleAck' not in main: errors.append('frontend_startup_ack_missing')
if 'OPENDECK_STARTUP_PROBE_FILE' not in menu: errors.append('menu_native_visibility_probe_missing')
if errors:
    print('OPENDECK_V250_TAURI_WINDOW_PERMISSION_CLOSURE=FAIL:' + ';'.join(errors))
    sys.exit(1)
print('OPENDECK_V250_TAURI_WINDOW_PERMISSION_CLOSURE=PASS')

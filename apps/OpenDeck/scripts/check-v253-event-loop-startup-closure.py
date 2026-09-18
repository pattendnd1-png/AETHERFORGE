#!/usr/bin/env python3
from pathlib import Path
import sys

root = Path(__file__).resolve().parents[1]
startup = (root / 'apps/opendeck-studio/src-tauri/src/startup.rs').read_text()
main = (root / 'apps/opendeck-studio/src/main.tsx').read_text()
menu = (root / 'scripts/check-v253-system-menu-launch.sh').read_text() if (root / 'scripts/check-v253-system-menu-launch.sh').exists() else ''

errors = []
prepare = startup.split('pub(crate) fn prepare_main_window', 1)[1].split('#[tauri::command]', 1)[0]
ack = startup.split('pub(crate) fn startup_visible_ack', 1)[1]

if '.is_visible()' in prepare:
    errors.append('prepare_main_window_must_not_assert_visibility_before_event_loop')
if 'window.show()' not in prepare:
    errors.append('native_show_request_missing')
if 'window.maximize()' in prepare:
    errors.append('native_startup_must_not_force_maximize')
if 'appMaximize' in main:
    errors.append('frontend_startup_must_not_force_maximize')
if '.is_visible()' not in ack:
    errors.append('frontend_visibility_ack_must_verify_tauri_visibility')
if 'startupVisibleAck' not in main:
    errors.append('frontend_visibility_ack_call_missing')
if 'appShow' not in main:
    errors.append('frontend_show_call_missing')
if 'OPENDECK_STARTUP_PROBE_FILE' not in menu:
    errors.append('menu_launch_probe_missing')

if errors:
    for error in errors:
        print(f'OPENDECK_V253_EVENT_LOOP_STARTUP_CLOSURE=FAIL:{error}')
    sys.exit(1)

print('OPENDECK_V253_EVENT_LOOP_STARTUP_CLOSURE=PASS')

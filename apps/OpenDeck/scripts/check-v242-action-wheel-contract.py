#!/usr/bin/env python3
from pathlib import Path
import sys

root = Path(__file__).resolve().parents[1]
checks = {
    'workspace_action_wheel_type': ('apps/opendeck-studio/src/model/workspace.ts', 'actionWheel: ActionWheel | null'),
    'action_wheel_active_helper': ('apps/opendeck-studio/src/model/action-wheel.ts', 'export function activeActionWheelEntry'),
    'action_wheel_step_helper': ('apps/opendeck-studio/src/model/action-wheel.ts', 'export function stepActionWheelIndex'),
    'action_wheel_catalog': ('apps/opendeck-studio/src/model/actions.ts', "id: 'editor.actionWheel'"),
    'reducer_create': ('apps/opendeck-studio/src/app/editor-store.ts', "type: 'CREATE_ACTION_WHEEL'"),
    'reducer_step': ('apps/opendeck-studio/src/app/editor-store.ts', "type: 'STEP_ACTION_WHEEL'"),
    'reducer_copy_ids': ('apps/opendeck-studio/src/app/editor-store.ts', "createId('action-wheel-entry')"),
    'app_rotate_route': ('apps/opendeck-studio/src/app/App.tsx', "type: 'STEP_ACTION_WHEEL'"),
    'app_create_route': ('apps/opendeck-studio/src/app/App.tsx', "id === 'editor.actionWheel'"),
    'touch_tap_execute': ('apps/opendeck-studio/src/app/hardware-events.ts', "wheelDial?.actionWheel"),
    'wheel_editor': ('apps/opendeck-studio/src/components/ActionWheelEditor.tsx', 'export function ActionWheelEditor'),
    'property_inspector': ('apps/opendeck-studio/src/components/PropertyInspector.tsx', 'Selected Wheel Action'),
    'dial_render': ('apps/opendeck-studio/src/components/DialControl.tsx', 'Action Wheel'),
    'qualification_demo': ('apps/opendeck-studio/src/qualify/demoWorkspace.ts', 'qualification-action-wheel-stream'),
    'rust_wheel_model': ('apps/opendeck-studio/src-tauri/src/editor.rs', 'action_wheel: Option<ActionWheel>'),
    'rust_wheel_validation': ('apps/opendeck-studio/src-tauri/src/editor.rs', 'rotateSelectPressExecute'),
    'rust_touch_overlay': ('apps/opendeck-studio/src-tauri/src/streamdeck/render.rs', 'dial.action_wheel.as_ref()'),
    'root_version': ('Cargo.toml', 'version = "2.0.42"'),
    'studio_version': ('apps/opendeck-studio/package.json', '"version": "2.0.42"'),
    'tauri_version': ('apps/opendeck-studio/src-tauri/Cargo.toml', 'version = "2.0.42"'),
    'tauri_conf_version': ('apps/opendeck-studio/src-tauri/tauri.conf.json', '"version": "2.0.42"'),
}
errors = []
for name, (rel, needle) in checks.items():
    path = root / rel
    if not path.exists():
        errors.append(f'{name}:missing-file:{rel}')
        continue
    text = path.read_text(errors='replace')
    if needle not in text:
        errors.append(f'{name}:missing:{needle}')

# Action Wheel and Dial Stack are intentionally mutually exclusive per dial.
store = (root / 'apps/opendeck-studio/src/app/editor-store.ts').read_text(errors='replace')
rust = (root / 'apps/opendeck-studio/src-tauri/src/editor.rs').read_text(errors='replace')
if "slot.dialStack || slot.actionWheel" not in store:
    errors.append('container_exclusivity:frontend')
if 'both a dial stack and action wheel' not in rust:
    errors.append('container_exclusivity:rust')

if errors:
    print('OPENDECK_V2_0_42_ACTION_WHEEL_CONTRACT=FAIL')
    for error in errors:
        print('ACTION_WHEEL_CONTRACT_ERROR=' + error)
    sys.exit(1)
print('OPENDECK_V2_0_42_ACTION_WHEEL_CONTRACT=PASS')

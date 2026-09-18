#!/usr/bin/env python3
from pathlib import Path
import sys

root = Path(__file__).resolve().parents[1]
checks = {
    'workspace_dial_stack_type': ('apps/opendeck-studio/src/model/workspace.ts', 'dialStack: DialStack | null'),
    'dial_stack_helper_active': ('apps/opendeck-studio/src/model/dial-stack.ts', 'export function activeDialStackEntry'),
    'dial_stack_helper_bindings': ('apps/opendeck-studio/src/model/dial-stack.ts', 'export function effectiveDialBindings'),
    'dial_stack_action_catalog': ('apps/opendeck-studio/src/model/actions.ts', "id: 'editor.dialStack'"),
    'reducer_create': ('apps/opendeck-studio/src/app/editor-store.ts', "type: 'CREATE_DIAL_STACK'"),
    'reducer_cycle': ('apps/opendeck-studio/src/app/editor-store.ts', "type: 'CYCLE_DIAL_STACK'"),
    'app_cycle_route': ('apps/opendeck-studio/src/app/App.tsx', "CYCLE_DIAL_STACK"),
    'stack_editor': ('apps/opendeck-studio/src/components/DialStackEditor.tsx', 'export function DialStackEditor'),
    'rust_stack_model': ('apps/opendeck-studio/src-tauri/src/editor.rs', 'dial_stack: Option<DialStack>'),
    'rust_touch_overlay': ('apps/opendeck-studio/src-tauri/src/streamdeck/render.rs', 'overlay_dial_stack_status'),
    'root_version': ('Cargo.toml', 'version = "2.0.30"'),
    'studio_version': ('apps/opendeck-studio/package.json', '"version": "2.0.30"'),
    'tauri_version': ('apps/opendeck-studio/src-tauri/Cargo.toml', 'version = "2.0.30"'),
}
errors=[]
for name,(rel,needle) in checks.items():
    path=root/rel
    if not path.exists():
        errors.append(f'{name}:missing-file:{rel}')
        continue
    text=path.read_text(errors='replace')
    if needle not in text:
        errors.append(f'{name}:missing:{needle}')
if errors:
    print('OPENDECK_V2_0_30_DIAL_STACK_CONTRACT=FAIL')
    for error in errors:
        print('DIAL_STACK_CONTRACT_ERROR='+error)
    sys.exit(1)
print('OPENDECK_V2_0_30_DIAL_STACK_CONTRACT=PASS')

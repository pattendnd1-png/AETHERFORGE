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
    'app_cycle_route': ('apps/opendeck-studio/src/app/App.tsx', 'CYCLE_DIAL_STACK'),
    'stack_editor': ('apps/opendeck-studio/src/components/DialStackEditor.tsx', 'export function DialStackEditor'),
    'rust_stack_model': ('apps/opendeck-studio/src-tauri/src/editor.rs', 'dial_stack: Option<DialStack>'),
    'rust_touch_overlay': ('apps/opendeck-studio/src-tauri/src/streamdeck/render.rs', 'overlay_dial_stack_status'),
}
errors=[]
for name,(rel,needle) in checks.items():
    text=(root/rel).read_text(errors='replace') if (root/rel).exists() else ''
    if needle not in text: errors.append(f'{name}:{rel}:{needle}')
if errors:
    print('OPENDECK_V233_DIAL_STACK_PRESERVATION=FAIL')
    for error in errors: print('DIAL_STACK_PRESERVATION_ERROR='+error)
    sys.exit(1)
print('OPENDECK_V233_DIAL_STACK_PRESERVATION=PASS')

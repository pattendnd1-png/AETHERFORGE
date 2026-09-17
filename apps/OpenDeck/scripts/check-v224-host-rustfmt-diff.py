#!/usr/bin/env python3
from pathlib import Path

render = Path('apps/opendeck-studio/src-tauri/src/streamdeck/render.rs').read_text()
runtime = Path('apps/opendeck-studio/src-tauri/src/streamdeck/runtime.rs').read_text()

checks = {
    'ADAPTIVE_RESOLVE_CALL_RUSTFMT_SINGLE_LINE': 'resolve_touch_presentation(&workspace.profiles[0].pages[0], Some("com.spotify.Client")),' in render,
    'SYNC_WORKSPACE_CALL_RUSTFMT_SHAPE': 'match sync_workspace_to_device(&device, workspace, active_app_id.as_deref())\n                        {' in runtime,
}
for name, ok in checks.items():
    print(f'OPENDECK_V224_RUSTFMT_DIFF_{name}={"PASS" if ok else "FAIL"}')
if not all(checks.values()):
    print('OPENDECK_V224_RUSTFMT_DIFF=FAIL')
    raise SystemExit(1)
print('OPENDECK_V224_RUSTFMT_DIFF=PASS')

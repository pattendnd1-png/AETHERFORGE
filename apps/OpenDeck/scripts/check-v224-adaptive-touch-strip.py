#!/usr/bin/env python3
from pathlib import Path

root = Path('.')
workspace = (root/'apps/opendeck-studio/src/model/workspace.ts').read_text()
device = (root/'apps/opendeck-studio/src/components/DeviceEditor.tsx').read_text()
prop = (root/'apps/opendeck-studio/src/components/PropertyInspector.tsx').read_text()
hardware = (root/'apps/opendeck-studio/src/app/hardware-events.ts').read_text()
bridge = (root/'apps/opendeck-studio/src/bridge.ts').read_text()
app = (root/'apps/opendeck-studio/src/app/App.tsx').read_text()
rust_editor = (root/'apps/opendeck-studio/src-tauri/src/editor.rs').read_text()
rust_render = (root/'apps/opendeck-studio/src-tauri/src/streamdeck/render.rs').read_text()
rust_lib = (root/'apps/opendeck-studio/src-tauri/src/lib.rs').read_text()
rust_runtime = (root/'apps/opendeck-studio/src-tauri/src/streamdeck/runtime.rs').read_text()
app_test = (root/'apps/opendeck-studio/src/App.test.tsx').read_text()
property_test = (root/'apps/opendeck-studio/src/components/PropertyInspector.test.tsx').read_text()
rust_editor_test = rust_editor
qualifier = (root/'scripts/qualify-v224-host.sh').read_text()

checks = {
    'MODEL_MODE_TYPE': "export type TouchStripMode = 'segmented' | 'unified' | 'adaptive';" in workspace,
    'MODEL_PRESENTATION_TYPE': "export type TouchStripPresentation = 'segmented' | 'unified';" in workspace,
    'MODEL_RESOLVER': 'resolveTouchStripPresentation' in workspace,
    'UNIFIED_SLOT': 'unifiedSlot' in workspace,
    'DEVICE_EFFECTIVE_PRESENTATION': 'touchPresentation' in device,
    'DEVICE_UNIFIED_CONTROL': 'UnifiedTouchControl' in device,
    'PROPERTY_MODE_CONTROL': 'Touch strip mode' in prop,
    'HARDWARE_PRESENTATION_ARG': 'presentation: TouchStripPresentation' in hardware,
    'HARDWARE_TOUCH_CONTEXT': 'touch?: TouchExecutionContext' in hardware,
    'ACTIVE_APP_BRIDGE': 'activeApplicationContext' in bridge,
    'ADAPTIVE_POLL_750': '750' in app and 'activeApplicationContext' in app,
    'KDTOOL_PROVIDER': 'kdotool' in rust_lib and 'getactivewindow' in rust_lib and 'getwindowclassname' in rust_lib,
    'XDTOOL_PROVIDER': 'xdotool' in rust_lib and 'getactivewindow' in rust_lib and 'getwindowclassname' in rust_lib,
    'RUST_TOUCH_CONFIG': 'TouchStripConfig' in rust_editor,
    'RUST_UNIFIED_RENDER': 'unified_slot' in rust_render and 'WINDOW_WIDTH' in rust_render,
    'RUNTIME_ACTIVE_APP': 'active_app_id' in rust_runtime,
    'APP_ADAPTIVE_TEST': 'switches an adaptive touch strip to unified for a matching active application' in app_test and 'com.spotify.Client' in app_test,
    'PROPERTY_ADAPTIVE_TEST': 'configures adaptive touch strip mode, fallback, and app rules' in property_test,
    'OLD_SCHEMA_DEFAULT_TEST': 'schema_one_workspace_without_touch_strip_defaults_to_segmented' in rust_editor_test,
    'QUALIFIER_V224_LABEL': 'OPENDECK_V2_0_24_RENDER_PERFORMANCE=START' in qualifier,
    'DIAL_STACKS_V225': 'OPENDECK_DIAL_STACKS=DEFERRED_TO_2.0.25' in qualifier,
}
for name, ok in checks.items():
    print(f'OPENDECK_V224_ADAPTIVE_TOUCH_{name}={"PASS" if ok else "FAIL"}')
if not all(checks.values()):
    raise SystemExit(1)
print('OPENDECK_V224_ADAPTIVE_TOUCH_STRIP=PASS')

#!/usr/bin/env python3
from pathlib import Path
import sys

root = Path(__file__).resolve().parents[1]
checks = []

def require(path, needles, label):
    text = (root / path).read_text()
    missing = [n for n in needles if n not in text]
    if missing:
        checks.append((False, f"{label}: missing {missing}"))
    else:
        checks.append((True, label))

require('apps/opendeck-studio/src/model/actions.ts', [
    "id: 'obs.toggleApp'", "label: 'Launch / Close OBS'"
], 'OBS_ACTION_DEFINITION')
require('apps/opendeck-studio/src/app/action-executor.ts', [
    "case 'obs.toggleApp':", "{ kind: 'obs.toggleApp' }"
], 'OBS_ACTION_RESOLVER')
require('apps/opendeck-studio/src/bridge.ts', [
    "obsToggleApp: () => invoke<'launched' | 'closed'>('obs_toggle_app')"
], 'OBS_TAURI_BRIDGE')
require('apps/opendeck-studio/src-tauri/src/lib.rs', [
    'fn obs_toggle_app()', 'obs_process_ids()', 'obs_toggle_app,'
], 'OBS_NATIVE_TOGGLE')
require('apps/opendeck-studio/src-tauri/src/lib.rs', [
    '"streamdeckplugin" | "sdplugin" => Some("plugin")',
    'if !(path.is_file() || path.is_dir())'
], 'PLUGIN_DOWNLOAD_DISCOVERY')
require('apps/opendeck-studio/src-tauri/src/plugin_host/mod.rs', [
    'code_path: Option<String>', 'code_path_win: Option<String>', 'code_path_mac: Option<String>',
    'fs::set_permissions(&target', 'action_state_image_path'
], 'PLUGIN_INSTALL_COMPATIBILITY')
require('apps/opendeck-studio/src-tauri/src/pack_manager.rs', [
    'pub(crate) fn active_icon_path', 'icons.json', 'deactivate_other_packs',
    'if active {', 'deactivate_other_packs(&pack_id)?;'
], 'ICON_PACK_DEVICE_THEME')
require('apps/opendeck-studio/src-tauri/src/streamdeck/render.rs', [
    'plugin_host::action_state_image_path', 'pack_manager::active_icon_path',
    'draw_builtin_icon', 'hardware_key_renderer_draws_an_icon_even_without_an_explicit_asset'
], 'DEVICE_ICON_RENDER_PATH')
require('apps/opendeck-studio/src/app/App.tsx', [
    'async function pushCurrentIconsToDevice()',
    'await bridge.streamdeckSyncWorkspace(workspaceRef.current, activeAppIdRef.current);',
    'Activated and pushed', 'Installed, activated, and pushed icon pack'
], 'ICON_PACK_IMMEDIATE_DEVICE_PUSH')
require('apps/opendeck-studio/src-tauri/Cargo.toml', [
    '"jpeg", "png", "gif", "webp"'
], 'DEVICE_ICON_FORMATS')
require('apps/opendeck-studio/src/App.test.tsx', [
    'obsToggleApp: vi.fn()', "mockResolvedValue('launched')"
], 'OBS_TEST_BRIDGE_MOCK')
require('apps/opendeck-studio/src/app/action-executor.test.ts', [
    "resolves the OBS launch/close toggle as a first-class action"
], 'OBS_ACTION_TEST')

failed = [msg for ok, msg in checks if not ok]
for ok, msg in checks:
    print(f"OPENDECK_V252_CONTRACT_{msg}={'PASS' if ok else 'FAIL'}")
if failed:
    print('OPENDECK_V252_DEVICE_ICON_PLUGIN_INSTALL_OBS_TOGGLE=FAIL')
    sys.exit(1)
print('OPENDECK_V252_DEVICE_ICON_PLUGIN_INSTALL_OBS_TOGGLE=PASS')

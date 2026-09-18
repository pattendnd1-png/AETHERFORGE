#!/usr/bin/env python3
from pathlib import Path
import json, sys
root=Path(__file__).resolve().parents[1]
checks={}
def has(rel,*needles):
    text=(root/rel).read_text()
    return all(n in text for n in needles)

def add(name,value): checks[name]=bool(value)

add('ROOT_VERSION', has('Cargo.toml','version = "2.0.49"'))
add('PACKAGE_VERSION', json.loads((root/'apps/opendeck-studio/package.json').read_text()).get('version')=='2.0.49')
conf=json.loads((root/'apps/opendeck-studio/src-tauri/tauri.conf.json').read_text())
window=conf['app']['windows'][0]
add('TAURI_VERSION', conf.get('version')=='2.0.49')
add('WINDOW_RESIZABLE', window.get('resizable') is True and window.get('minWidth')==640 and window.get('minHeight')==480)
add('RESIZE_PERMISSION', has('apps/opendeck-studio/src-tauri/capabilities/default.json','core:window:allow-start-resize-dragging'))
add('EIGHT_RESIZE_DIRECTIONS', has('apps/opendeck-studio/src/components/WindowResizeFrame.tsx',"'North'","'South'","'East'","'West'","'NorthEast'","'NorthWest'","'SouthEast'","'SouthWest'"))
add('NO_FORCED_STARTUP_MAXIMIZE', 'appMaximize()' not in (root/'apps/opendeck-studio/src/main.tsx').read_text())
add('PROFILE_WORKSPACE', has('apps/opendeck-studio/src/components/ProfilesWorkspace.tsx','Select a profile, then activate it','Activate','pluginReadonly'))
add('PROFILE_ACTIVE_REDUCER', has('apps/opendeck-studio/src/app/editor-store.ts',"case 'SET_ACTIVE_PROFILE'",'workspace.active_profile_id = profile.id'))
add('PLUGIN_PROFILE_OWNERSHIP', has('apps/opendeck-studio/src/model/workspace.ts','pluginOwnerUuid','pluginReadonly') and has('apps/opendeck-studio/src-tauri/src/editor.rs','plugin_owner_uuid','plugin_readonly'))
add('PLUGIN_MANAGER_LIFECYCLE', has('apps/opendeck-studio/src/components/PluginManager.tsx','Activate','Deactivate','Enable','Disable','Restart','Remove','Bundled profiles'))
add('DUAL_PLUGIN_MANIFESTS', has('apps/opendeck-studio/src-tauri/src/plugin_host/mod.rs','ElgatoManifest','OpenDeckManifest','manifest.json','opendeck-plugin.json'))
add('PROCESS_ISOLATION', has('apps/opendeck-studio/src-tauri/src/plugin_host/mod.rs','process::Command','Stdio::null()','RuntimeHandle'))
add('WS_REGISTRATION', has('apps/opendeck-studio/src-tauri/src/plugin_host/mod.rs','-port','-pluginUUID','-registerEvent','registerPlugin','accept_async'))
add('NODE_NATIVE_WINE', has('apps/opendeck-studio/src-tauri/src/plugin_host/mod.rs','Some("node")','Some("native")','Some("wine")','wine64'))
add('PROPERTY_INSPECTOR', has('apps/opendeck-studio/src/components/PluginPropertyInspector.tsx','connectElgatoStreamDeckSocket') or has('apps/opendeck-studio/src-tauri/src/plugin_host/mod.rs','registerPropertyInspector','connectElgatoStreamDeckSocket'))
add('SETTINGS_AND_RESOURCES', has('apps/opendeck-studio/src-tauri/src/plugin_host/mod.rs','getSettings','setSettings','getGlobalSettings','setGlobalSettings','getResources','setResources','getSecrets'))
add('FEEDBACK_COMMANDS', has('apps/opendeck-studio/src-tauri/src/plugin_host/mod.rs','setTitle','setImage','setState','setFeedback','setFeedbackLayout','setTriggerDescription','showOk','showAlert'))
add('PROFILE_SWITCH_RESTRICTED', has('apps/opendeck-studio/src/app/App.tsx','candidate.pluginOwnerUuid === pluginUuid','previousPluginProfileRef'))
add('BUNDLED_PROFILE_IMPORT', has('apps/opendeck-studio/src-tauri/src/plugin_host/mod.rs','plugin_import_bundled_profile','DEVICE_TYPE_STREAM_DECK_PLUS') and has('apps/opendeck-studio/src/app/App.tsx','profile.autoInstall && profile.deviceType === 7'))
add('MULTI_ACTION', has('apps/opendeck-studio/src/model/actions.ts',"'editor.multiAction'") and has('apps/opendeck-studio/src/components/MultiActionEditor.tsx','isInMultiAction=true'))
add('KEY_LOGIC', has('apps/opendeck-studio/src/model/actions.ts',"'editor.keyLogic'") and has('apps/opendeck-studio/src/components/KeyLogicEditor.tsx','Double Press','Press & Hold'))
add('STREAMDECK_PLUS_EVENTS', has('apps/opendeck-studio/src-tauri/src/plugin_host/mod.rs','dialDown','dialUp','dialRotate','touchTap','Keypad','Encoder'))
add('AUTO_STATE_TOGGLE', has('apps/opendeck-studio/src-tauri/src/plugin_host/mod.rs','disable_automatic_states','maybe_toggle_automatic_state'))
add('ACTION_VISIBILITY_CAPS', has('apps/opendeck-studio/src-tauri/src/plugin_host/mod.rs','supported_in_multi_actions','supported_in_key_logic_actions','visible_in_actions_list'))
add('PLUGIN_IMAGE_PATHS', has('apps/opendeck-studio/src-tauri/src/plugin_host/mod.rs','plugin image path must stay inside the plugin directory','data:image/svg+xml'))
add('PLUGIN_HOST_RUST_COMPILE_CLOSURE', (root/'scripts/check-v249-plugin-host-rust-compile-closure.py').is_file() and has('scripts/qualify-v249-host.sh','PLUGIN_HOST_RUST_COMPILE_CLOSURE'))
add('PLUGIN_HOST_STRICT_CLIPPY_CLOSURE', (root/'scripts/check-v249-plugin-host-strict-clippy-closure.py').is_file() and has('scripts/qualify-v249-host.sh','gate "PLUGIN_HOST_STRICT_CLIPPY_CLOSURE"'))
add('QUALIFICATION_ENV_CLOSURE', (root/'scripts/check-v249-qualification-env-closure.py').is_file() and has('scripts/qualify-v249-host.sh','gate "QUALIFICATION_ENV_CLOSURE"'))
add('FLUID_WINDOW_FILL_CLOSURE', (root/'scripts/check-v249-fluid-window-fill-closure.py').is_file() and has('scripts/qualify-v249-host.sh','gate "FLUID_WINDOW_FILL_CLOSURE"'))
add('SYSTEM_STAGE', (root/'scripts/stage-v249-system-wide.sh').is_file())
add('SYSTEM_ACTIVATE', (root/'scripts/activate-v249-system-wide.sh').is_file())
add('SYSTEM_ROLLBACK', (root/'scripts/rollback-v249-system-wide.sh').is_file())
add('SYSTEM_UNINSTALL', (root/'scripts/uninstall-v249-system-wide.sh').is_file())
failed=[name for name,ok in checks.items() if not ok]
for name,ok in checks.items(): print(f'{name}={"PASS" if ok else "FAIL"}')
if failed:
    print('OPENDECK_V249_FULL_PARITY_CONTRACT=FAIL:'+','.join(failed))
    sys.exit(1)
print('OPENDECK_V249_FULL_PARITY_CONTRACT=PASS')

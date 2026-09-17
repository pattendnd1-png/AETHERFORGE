from pathlib import Path
import json
import sys

root = Path(__file__).resolve().parents[1]
studio = root / 'apps/opendeck-studio'
frontend_root = studio / 'src'
app = (frontend_root / 'app/App.tsx').read_text()
store = (frontend_root / 'app/editor-store.ts').read_text()
workspace = (frontend_root / 'model/workspace.ts').read_text()
bridge = (frontend_root / 'bridge.ts').read_text()
css = (frontend_root / 'styles.css').read_text()
assets_rs = (studio / 'src-tauri/src/assets.rs').read_text()
editor_rs = (studio / 'src-tauri/src/editor.rs').read_text()
lib_rs = (studio / 'src-tauri/src/lib.rs').read_text()
pkg = json.loads((studio / 'package.json').read_text())
tauri = json.loads((studio / 'src-tauri/tauri.conf.json').read_text())
tsnode = json.loads((studio / 'tsconfig.node.json').read_text())
root_cargo = (root / 'Cargo.toml').read_text()
app_cargo = (studio / 'src-tauri/Cargo.toml').read_text()
protocol_rs = (studio / 'src-tauri/src/streamdeck/protocol.rs').read_text()
render_rs = (studio / 'src-tauri/src/streamdeck/render.rs').read_text()
runtime_rs = (studio / 'src-tauri/src/streamdeck/runtime.rs').read_text()
hardware_events = (frontend_root / 'app/hardware-events.ts').read_text()
udev_rules = (root / 'packaging/70-opendeck-streamdeck.rules').read_text()
probe = (root / 'scripts/check-streamdeck-plus.sh').read_text()
action_library = (frontend_root / 'components/ActionLibrary.tsx').read_text()
sidebar = (frontend_root / 'components/AppSidebar.tsx').read_text()
device = (frontend_root / 'components/DeviceEditor.tsx').read_text()
inspector = (frontend_root / 'components/PropertyInspector.tsx').read_text()

required_files = [
    frontend_root / 'model/workspace.ts', frontend_root / 'model/actions.ts',
    frontend_root / 'app/editor-store.ts', frontend_root / 'components/DeviceEditor.tsx',
    frontend_root / 'components/ActionLibrary.tsx', frontend_root / 'components/PropertyInspector.tsx',
    frontend_root / 'components/AssetBrowser.tsx', frontend_root / 'components/ProfileManager.tsx',
    frontend_root / 'components/AppSidebar.tsx', frontend_root / 'components/VirtualActionList.tsx',
    studio / 'src-tauri/src/editor.rs', studio / 'src-tauri/src/assets.rs',
    studio / 'src-tauri/src/streamdeck/protocol.rs', studio / 'src-tauri/src/streamdeck/render.rs',
    studio / 'src-tauri/src/streamdeck/runtime.rs', frontend_root / 'app/hardware-events.ts',
    root / 'packaging/70-opendeck-streamdeck.rules', root / 'scripts/check-streamdeck-plus.sh',
    studio / 'src-tauri/icons/icon.png',
]

junk_names = {'node_modules', 'dist', 'target', '__pycache__'}
junk_files = []
for path in root.rglob('*'):
    if '.git' in path.parts:
        continue
    if path.is_dir() and path.name in junk_names:
        junk_files.append(path.relative_to(root).as_posix() + '/')
    if path.is_file() and (path.suffix in {'.pyc', '.pyo', '.tsbuildinfo'} or path.name in {'vite.config.js', 'vite.config.d.ts'}):
        junk_files.append(path.relative_to(root).as_posix())

legacy_local_storage_refs = [line.strip() for line in app.splitlines() if 'localStorage' in line]
allowed_legacy_storage = all('opendeck-v2.keys' in line or 'localStorage.removeItem' in line for line in legacy_local_storage_refs)

checks = {
    'OPENDECK_V227_VERSION': (
        pkg.get('version') == '2.0.27' and tauri.get('version') == '2.0.27'
        and 'version = "2.0.27"' in root_cargo and 'version = "2.0.27"' in app_cargo
    ),
    'OPENDECK_V227_QUALIFICATION_MODE': all(marker in app for marker in ['qualification?: QualificationContext', 'qualification.enabled', 'qualification.phase', 'createQualificationWorkspace']),
    'OPENDECK_V227_RENDER_SHELL': all(marker in app for marker in ['<AppSidebar', 'data-layout-region="main"']) and all(marker in sidebar for marker in ['Buttons', 'Dials', 'Touch Strip', 'Profiles', 'Plugins', 'Settings']),
    'OPENDECK_V227_DEVICE_PRESENTATION': all(marker in device for marker in ['streamdeck-device', 'streamdeck-touch-display', 'streamdeck-dials', 'data-qualify-element="device"']),
    'OPENDECK_V227_ACTION_LIBRARY': all(marker in action_library for marker in ['VirtualActionList', 'action-library-title', "supportsControl(item.id, 'dial')"]),
    'OPENDECK_V227_CONFIGURATION': all(marker in inspector for marker in ['Configure:', 'interaction-rail', 'Assigned Action']),
    'OPENDECK_V227_DRAGONGLASS': all(marker in css for marker in ['--sidebar-width: 238px', '--action-width: 388px', '--device-width: 735px', 'backdrop-filter', '.dial-ring', '.streamdeck-touch-display']),
    'OPENDECK_V203_HARDWARE_BRIDGE': all(marker in bridge for marker in ['streamdeckStatus', 'streamdeckSyncWorkspace', 'streamdeckSetBrightness']),
    'OPENDECK_V202_EDITOR_MODEL': all(path.exists() for path in required_files[:3]) and 'interface Workspace' in workspace and 'EditorAction' in store,
    'OPENDECK_V202_16_CONTROLS': all(marker in workspace for marker in ['length: 8', 'length: 4']) and all(marker in app for marker in ['DeviceEditor', 'PropertyInspector', 'ActionLibrary']),
    'OPENDECK_V202_RUST_PERSISTENCE': all(marker in editor_rs for marker in ['editor_load_workspace', 'editor_save_workspace', 'workspace.json', 'workspace.backup.json']) and all(marker in bridge for marker in ['editorLoadWorkspace', 'editorSaveWorkspace']),
    'OPENDECK_V202_ASSET_LIBRARY': all(marker in assets_rs for marker in ['editor_import_asset', 'editor_list_assets', 'editor_asset_data_urls', 'icon-packs']) and 'AssetBrowser' in app,
    'OPENDECK_V202_UNDO_REDO': all(marker in store for marker in ["type: 'UNDO'", "type: 'REDO'", 'HISTORY_LIMIT']) and all(marker in app for marker in ['canUndo(state)', 'canRedo(state)']),
    'OPENDECK_V202_NO_LOCALSTORAGE_AUTHORITY': allowed_legacy_storage and 'editorSaveWorkspace' in app and 'persistKeys' not in app and 'loadKeys' not in app,
    'OPENDECK_V202_DIRECT_MANIPULATION': all(marker in app for marker in ['COPY_CONTROL', 'MOVE_CONTROL', 'ASSIGN_ACTION_TO_CONTROL']) and 'application/x-opendeck-action' in action_library and 'application/x-opendeck-control' in device,
    'OPENDECK_V207_DIALS_ACTION_FILTER': "supportsControl(item.id, 'dial') || supportsControl(item.id, 'touch')" not in action_library and "supportsControl(item.id, 'dial')" in action_library,
    'OPENDECK_V203_HID_PROTOCOL': all(marker in protocol_rs for marker in ['ELGATO_VID', 'STREAM_DECK_PLUS_PID', 'parse_input_report', 'button_image_reports', 'window_image_reports']),
    'OPENDECK_V203_HARDWARE_RENDERER': all(marker in render_rs for marker in ['render_workspace', 'KEY_WIDTH', 'WINDOW_WIDTH', 'encode_jpeg']) and 'image = { version = "0.25"' in app_cargo,
    'OPENDECK_V203_SINGLE_OWNER_RUNTIME': all(marker in runtime_rs for marker in ['opendeck-streamdeck-plus', 'read_timeout', 'opendeck://hardware-input', 'opendeck://hardware-status', 'SyncWorkspace', 'SetBrightness']) and 'hidapi = { version = "2.6.7"' in app_cargo,
    'OPENDECK_V204_OPEN_DEVICE_RESULT_TYPE': ('struct OpenedDevice' in runtime_rs and 'fn open_device() -> Result<Option<OpenedDevice>, String>' in runtime_rs),
    'OPENDECK_V203_PHYSICAL_ACTION_DISPATCH': all(marker in hardware_events for marker in ['keyDown', 'dialRotate', 'touchTap', 'MAX_ROTATION_ACTIONS']) and all(marker in app for marker in ["origin === 'test'", "'hardware'", 'streamdeckSyncWorkspace']),
    'OPENDECK_V203_UDEV_UACCESS': all(marker in udev_rules for marker in ['SUBSYSTEM=="usb"', 'SUBSYSTEM=="hidraw"', '0fd9', '0084', 'TAG+="uaccess"']) and '0666' not in udev_rules,
    'OPENDECK_V202_SERVICE_PRESERVATION': all(marker in lib_rs for marker in ['GetVersion', 'GetSceneList', 'ToggleStream', 'oauth2/device', 'https://marketplace.elgato.com']),
    'OPENDECK_V202_NO_BACKGROUND_SERVICE': 'systemctl' not in lib_rs and 'opendeck-daemon' not in '\n'.join(path.as_posix() for path in root.rglob('*') if '.git' not in path.parts),
    'OPENDECK_V202_NO_AUTOSTART': 'autostart' not in lib_rs.lower(),
    'OPENDECK_V202_TS_NODE_NOEMIT': tsnode.get('compilerOptions', {}).get('allowImportingTsExtensions') is True and tsnode.get('compilerOptions', {}).get('noEmit') is True,
    'OPENDECK_V202_TAURI_ICON': (studio / 'src-tauri/icons/icon.png').read_bytes().startswith(b'\x89PNG\r\n\x1a\n'),
    'OPENDECK_V227_SOURCE_HYGIENE': not junk_files,
}

for key, passed in checks.items():
    print(f'{key}={"PASS" if passed else "FAIL"}')
if junk_files:
    for path in sorted(set(junk_files)):
        print(f'OPENDECK_SOURCE_HYGIENE_REJECT={path}')
bad = [key for key, passed in checks.items() if not passed]
if bad:
    print('OPENDECK_V2_0_27_SOURCE_CONTRACT=FAIL:' + ','.join(bad))
    sys.exit(1)
print('OPENDECK_V2_0_27_SOURCE_CONTRACT=PASS')

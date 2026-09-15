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

required_files = [
    frontend_root / 'model/workspace.ts',
    frontend_root / 'model/actions.ts',
    frontend_root / 'app/editor-store.ts',
    frontend_root / 'components/DeviceEditor.tsx',
    frontend_root / 'components/ActionLibrary.tsx',
    frontend_root / 'components/PropertyInspector.tsx',
    frontend_root / 'components/AssetBrowser.tsx',
    frontend_root / 'components/ProfileManager.tsx',
    studio / 'src-tauri/src/editor.rs',
    studio / 'src-tauri/src/assets.rs',
    studio / 'src-tauri/icons/icon.png',
]

junk_names = {'node_modules', 'dist', 'target', '__pycache__'}
junk_files = []
for path in root.rglob('*'):
    if '.git' in path.parts:
        continue
    if path.is_dir() and path.name in junk_names:
        junk_files.append(path.relative_to(root).as_posix() + '/')
    if path.is_file() and (
        path.suffix in {'.pyc', '.pyo', '.tsbuildinfo'}
        or path.name in {'vite.config.js', 'vite.config.d.ts'}
    ):
        junk_files.append(path.relative_to(root).as_posix())

legacy_local_storage_refs = [line.strip() for line in app.splitlines() if 'localStorage' in line]
allowed_legacy_storage = all('opendeck-v2.keys' in line or 'localStorage.removeItem' in line for line in legacy_local_storage_refs)

checks = {
    'OPENDECK_V202_VERSION': (
        pkg.get('version') == '2.0.2'
        and tauri.get('version') == '2.0.2'
        and 'version = "2.0.2"' in root_cargo
        and 'version = "2.0.2"' in app_cargo
    ),
    'OPENDECK_V202_EDITOR_MODEL': all(path.exists() for path in required_files[:3]) and 'interface Workspace' in workspace and 'EditorAction' in store,
    'OPENDECK_V202_16_CONTROLS': all(marker in workspace for marker in ['length: 8', 'length: 4']) and all(marker in app for marker in ['DeviceEditor', 'PropertyInspector', 'ActionLibrary']),
    'OPENDECK_V202_RUST_PERSISTENCE': all(marker in editor_rs for marker in ['editor_load_workspace', 'editor_save_workspace', 'workspace.json', 'workspace.backup.json']) and all(marker in bridge for marker in ['editorLoadWorkspace', 'editorSaveWorkspace']),
    'OPENDECK_V202_ASSET_LIBRARY': all(marker in assets_rs for marker in ['editor_import_asset', 'editor_list_assets', 'editor_asset_data_urls', 'icon-packs']) and 'AssetBrowser' in app,
    'OPENDECK_V202_UNDO_REDO': all(marker in store for marker in ["type: 'UNDO'", "type: 'REDO'", 'HISTORY_LIMIT']) and all(marker in app for marker in ['canUndo(state)', 'canRedo(state)']),
    'OPENDECK_V202_NO_LOCALSTORAGE_AUTHORITY': allowed_legacy_storage and 'editorSaveWorkspace' in app and 'persistKeys' not in app and 'loadKeys' not in app,
    'OPENDECK_V202_DIRECT_MANIPULATION': all(marker in app for marker in ['COPY_CONTROL', 'MOVE_CONTROL', 'ASSIGN_ACTION_TO_CONTROL']) and 'application/x-opendeck-action' in (frontend_root / 'components/ActionLibrary.tsx').read_text() and 'application/x-opendeck-control' in (frontend_root / 'components/DeviceEditor.tsx').read_text(),
    'OPENDECK_V202_CONTEXTUAL_CUSTOMIZATION': all(marker in (frontend_root / 'inspector/AppearanceInspector.tsx').read_text() for marker in ['Choose icon', 'Choose background', 'Font', 'Background']) and all(marker in (frontend_root / 'inspector/ActionInspector.tsx').read_text() for marker in ['pageId', 'profileId']),
    'OPENDECK_V202_ACTION_EXECUTION': (frontend_root / 'app/action-executor.ts').exists() and 'resolveActionExecution' in app and 'Test Action' in (frontend_root / 'inspector/ActionInspector.tsx').read_text() and all(marker in app for marker in ['obsToggleStream', 'obsToggleRecord', 'obsToggleMute', 'obsSetScene', 'SET_ACTIVE_PAGE', 'SET_ACTIVE_PROFILE']),
    'OPENDECK_V202_WINDOWS_DENSITY': all(marker in css for marker in ['44px', '--action-panel-width', '--inspector-height', 'action-resize-handle', 'inspector-resize-handle']) and all(marker in app for marker in ['actionPanelCollapsed', 'inspectorCollapsed', 'UPDATE_PREFERENCES']),
    'OPENDECK_V202_SERVICE_PRESERVATION': all(marker in lib_rs for marker in ['GetVersion', 'GetSceneList', 'ToggleStream', 'oauth2/device', 'https://marketplace.elgato.com']),
    'OPENDECK_V202_NO_BACKGROUND_SERVICE': 'systemctl' not in lib_rs and 'opendeck-daemon' not in '\n'.join(path.as_posix() for path in root.rglob('*') if '.git' not in path.parts),
    'OPENDECK_V202_NO_AUTOSTART': 'autostart' not in lib_rs.lower(),
    'OPENDECK_V202_TS_NODE_NOEMIT': tsnode.get('compilerOptions', {}).get('allowImportingTsExtensions') is True and tsnode.get('compilerOptions', {}).get('noEmit') is True,
    'OPENDECK_V202_TAURI_ICON': (studio / 'src-tauri/icons/icon.png').read_bytes().startswith(b'\x89PNG\r\n\x1a\n'),
    'OPENDECK_V202_SOURCE_HYGIENE': not junk_files,
}

for key, passed in checks.items():
    print(f'{key}={"PASS" if passed else "FAIL"}')

if junk_files:
    for path in sorted(set(junk_files)):
        print(f'OPENDECK_SOURCE_HYGIENE_REJECT={path}')

bad = [key for key, passed in checks.items() if not passed]
if bad:
    print('OPENDECK_V2_0_2_SOURCE_CONTRACT=FAIL:' + ','.join(bad))
    sys.exit(1)

print('OPENDECK_V2_0_2_SOURCE_CONTRACT=PASS')

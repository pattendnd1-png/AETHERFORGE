#!/usr/bin/env python3
from pathlib import Path
import sys

root = Path(__file__).resolve().parents[1]
manager = (root / 'apps/opendeck-studio/src/components/PluginManager.tsx').read_text()
manager_tests = (root / 'apps/opendeck-studio/src/components/PluginManager.test.tsx').read_text()
plugin = (root / 'apps/opendeck-studio/src-tauri/src/plugin_host/mod.rs').read_text()
pack = (root / 'apps/opendeck-studio/src-tauri/src/pack_manager.rs').read_text()
assets = (root / 'apps/opendeck-studio/src-tauri/src/assets.rs').read_text()
app = (root / 'apps/opendeck-studio/src/app/App.tsx').read_text()
css = (root / 'apps/opendeck-studio/src/styles.css').read_text()
workspace = (root / 'apps/opendeck-studio/src/model/workspace.ts').read_text()

checks = {
    'PLUGIN_UI_SERIALIZES_MUTATIONS': 'const [busyLabel, setBusyLabel]' in manager and 'const busyRef = useRef(false);' in manager and manager.count('if (busyRef.current) return;') >= 2 and 'aria-busy={busy}' in manager,
    'PLUGIN_UI_LOCKS_SELECTION_WHILE_BUSY': manager.count('disabled={busy}') >= 8,
    'PLUGIN_UI_BUSY_REGRESSION_TEST': 'serializes extension mutations and exposes a busy state' in manager_tests and "toHaveAttribute('aria-busy', 'true')" in manager_tests and 'toHaveBeenCalledTimes(1)' in manager_tests,
    'PLUGIN_BACKEND_SERIALIZES_LIFECYCLE': 'operation_lock: Arc<AsyncMutex<()>>' in plugin and plugin.count('operation_lock.lock().await') >= 6,
    'PACK_IO_OFF_EVENT_LOOP': pack.count('spawn_blocking') >= 4 and 'PACK_OPERATION_LOCK' in pack and 'with_pack_operation' in pack,
    'PACK_ACTIVATION_PRESERVES_CURRENT_ON_TARGET_MOVE_FAILURE': pack.index('fs::rename(&inactive_path, &active_path)') < pack.index('deactivate_other_packs(&pack_id)?;', pack.index('fn icon_pack_set_active_blocking')),
    'PACK_COUNTS_CACHED': 'item_count: Option<usize>' in pack and 'meta.item_count = Some(icon_count(&staging));' in pack,
    'ASSET_SCAN_OFF_EVENT_LOOP': 'async fn editor_list_assets' in assets and 'spawn_blocking(list_assets_blocking)' in assets,
    'ASSET_PREVIEW_OFF_EVENT_LOOP': 'async fn editor_asset_data_urls' in assets and 'asset_data_urls_blocking' in assets,
    'PACK_PUSH_PRECEDES_BACKGROUND_ASSET_REFRESH': 'await pushCurrentIconsToDevice();\n    setStatus' in app and 'void refreshAssets().catch' in app,
    'COMPACT_HEADER': '--header-height: 76px;' in css,
    'COMPACT_SIDEBAR': '--sidebar-width: 188px;' in css,
    'COMPACT_ACTION_LIBRARY': '--action-width: 320px;' in css,
    'COMPACT_DEVICE_SCALE': '--device-width: 690px;' in css and '--device-zoom: .90;' in css,
    'COMPACT_INSPECTOR': 'const INSPECTOR_MIN = 180;' in app and 'const INSPECTOR_MAX = 280;' in app and 'inspectorHeight: 260' in workspace,
}

failed = [name for name, ok in checks.items() if not ok]
for name, ok in checks.items():
    print(f'OPENDECK_V255_{name}={"PASS" if ok else "FAIL"}')
if failed:
    print('OPENDECK_V255_EXTENSION_STABILITY_COMPACT_UI=FAIL:' + ','.join(failed))
    sys.exit(1)
print('OPENDECK_V255_EXTENSION_STABILITY_COMPACT_UI=PASS')

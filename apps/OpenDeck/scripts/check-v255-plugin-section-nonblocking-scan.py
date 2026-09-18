#!/usr/bin/env python3
from pathlib import Path
root = Path(__file__).resolve().parents[1]
app = (root/'apps/opendeck-studio/src/app/App.tsx').read_text()
backend = (root/'apps/opendeck-studio/src-tauri/src/lib.rs').read_text()
scan_block = backend.split('fn scan_marketplace_downloads_blocking()', 1)[1].split('#[tauri::command]', 1)[0]
test = (root/'apps/opendeck-studio/src/App.test.tsx').read_text()
visual = (root/'scripts/check-v255-visual-metrics.py').read_text()
checks = {
    'NO_AUTO_SCAN_SIDEBAR': "if (next === 'plugins') { setPanel('none'); void refreshPlugins(); void refreshIconPacks(); }" in app,
    'NO_AUTO_SCAN_MARKETPLACE_NAV': "onMarketplace={() => { setSection('plugins'); setPanel('none'); void refreshPlugins(); void refreshIconPacks(); }}" in app,
    'SCAN_INFLIGHT_GUARD': 'marketplaceScanBusyRef.current' in app,
    'MANUAL_SCAN_REMAINS': '>Scan Downloads</button>' in (root/'apps/opendeck-studio/src/components/PluginManager.tsx').read_text(),
    'SCAN_ASYNC_COMMAND': '#[tauri::command]\nasync fn scan_marketplace_downloads()' in backend,
    'SCAN_OFF_EVENT_LOOP': 'spawn_blocking(scan_marketplace_downloads_blocking)' in backend,
    'SCAN_DOWNLOADS_ONLY': 'let root = home_dir()?.join("Downloads")' in scan_block,
    'INSTALLED_PACK_TREE_EXCLUDED': '.local/share/opendeck/icon-packs' not in scan_block,
    'SCAN_DEPTH_BOUNDED': '.max_depth(3)' in backend,
    'SCAN_PRUNES_TARGET': '| "target"' in backend,
    'SCAN_PRUNES_NODE_MODULES': '| "node_modules"' in backend,
    'SCAN_STOPS_AT_PACKAGE_DIR': 'walker.skip_current_dir();' in backend,
    'SCAN_RESULT_CAP': 'const MAX_RESULTS: usize = 512;' in backend,
    'FRONTEND_NO_AUTOSCAN_TEST': 'opens Plugins & Packs without automatically scanning Downloads' in test,
    'COMPACT_VISUAL_MIN_240': "configuration.get('height', 0) < 240" in visual,
}
for name, ok in checks.items():
    print(f'OPENDECK_V255_{name}={"PASS" if ok else "FAIL"}')
failed=[name for name,ok in checks.items() if not ok]
if failed:
    print('OPENDECK_V255_PLUGIN_SECTION_NONBLOCKING_SCAN=FAIL:'+','.join(failed))
    raise SystemExit(1)
print('OPENDECK_V255_PLUGIN_SECTION_NONBLOCKING_SCAN=PASS')

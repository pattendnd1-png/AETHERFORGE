#!/usr/bin/env python3
from pathlib import Path
import json, sys
root=Path(__file__).resolve().parents[1]
checks={}
def has(rel, needle):
    p=root/rel
    return p.is_file() and needle in p.read_text()
checks['ROOT_VERSION']=has('Cargo.toml','version = "2.0.55"')
checks['PACKAGE_VERSION']=json.loads((root/'apps/opendeck-studio/package.json').read_text()).get('version')=='2.0.55'
checks['TAURI_VERSION']=json.loads((root/'apps/opendeck-studio/src-tauri/tauri.conf.json').read_text()).get('version')=='2.0.55'
checks['PACK_MANAGER_EXISTS']=(root/'apps/opendeck-studio/src-tauri/src/pack_manager.rs').is_file()
checks['PACK_LIST_COMMAND']=has('apps/opendeck-studio/src-tauri/src/pack_manager.rs','pub(crate) async fn icon_pack_list')
checks['PACK_INSTALL_COMMAND']=has('apps/opendeck-studio/src-tauri/src/pack_manager.rs','pub(crate) async fn icon_pack_install')
checks['PACK_ACTIVATE_COMMAND']=has('apps/opendeck-studio/src-tauri/src/pack_manager.rs','pub(crate) async fn icon_pack_set_active')
checks['PACK_REMOVE_COMMAND']=has('apps/opendeck-studio/src-tauri/src/pack_manager.rs','pub(crate) async fn icon_pack_remove')
checks['PACK_ACTIVE_ROOT']=has('apps/opendeck-studio/src-tauri/src/pack_manager.rs','.local/share/opendeck/icon-packs')
checks['PACK_INACTIVE_ROOT']=has('apps/opendeck-studio/src-tauri/src/pack_manager.rs','.local/share/opendeck/icon-packs-disabled')
checks['PACK_SAFE_ZIP']=has('apps/opendeck-studio/src-tauri/src/pack_manager.rs','enclosed_name()')
checks['PACK_COMMANDS_REGISTERED']=all(has('apps/opendeck-studio/src-tauri/src/lib.rs', n) for n in [
    'pack_manager::icon_pack_list','pack_manager::icon_pack_install','pack_manager::icon_pack_set_active','pack_manager::icon_pack_remove'])
checks['BRIDGE_PACK_API']=all(has('apps/opendeck-studio/src/bridge.ts', n) for n in ['iconPackList','iconPackInstall','iconPackSetActive','iconPackRemove'])
checks['PLUGIN_DIRECT_ACTIVATION_AUTO_ENABLE']=has('apps/opendeck-studio/src-tauri/src/plugin_host/mod.rs','plugin.enabled = true;') and has('apps/opendeck-studio/src-tauri/src/plugin_host/mod.rs','cloned.start_plugin(&plugin_uuid).await')
checks['SELECTABLE_PLUGIN_UI']=has('apps/opendeck-studio/src/components/PluginManager.tsx',"setSelection({ kind: 'plugin'") and has('apps/opendeck-studio/src/components/PluginManager.tsx','Activate Plugin')
checks['SELECTABLE_PACK_UI']=has('apps/opendeck-studio/src/components/PluginManager.tsx',"setSelection({ kind: 'icon-pack'") and has('apps/opendeck-studio/src/components/PluginManager.tsx','Activate Pack')
checks['SELECTABLE_DOWNLOAD_UI']=has('apps/opendeck-studio/src/components/PluginManager.tsx',"setSelection({ kind: 'download'") and has('apps/opendeck-studio/src/components/PluginManager.tsx','Install & Activate')
checks['PROFILE_IMPORT_ACTIVATE']=has('apps/opendeck-studio/src/app/App.tsx',"normalized.endsWith('.streamdeckprofile')") and has('apps/opendeck-studio/src/app/App.tsx','await importProfile(path, activate)')
checks['ICON_PACK_INSTALL_ACTIVATE']=has('apps/opendeck-studio/src/app/App.tsx',"normalized.endsWith('.streamdeckiconpack')") and has('apps/opendeck-studio/src/app/App.tsx','await installIconPack(path)')
checks['ASSET_REFRESH_AFTER_PACK_CHANGE']=has('apps/opendeck-studio/src/app/App.tsx','void refreshAssets().catch') and has('apps/opendeck-studio/src/app/App.tsx','await refreshIconPacks()')
checks['MARKETPLACE_ROUTES_TO_MANAGER']=has('apps/opendeck-studio/src/app/App.tsx',"onMarketplace={() => { setSection('plugins')")
checks['UI_REGRESSION_TEST']=(root/'apps/opendeck-studio/src/components/PluginManager.test.tsx').is_file() and all(has('apps/opendeck-studio/src/components/PluginManager.test.tsx', n) for n in ['activates it directly','activates it','Install & Activate'])
checks['QUALIFIER_GATE']=has('scripts/qualify-v255-host.sh','PLUGIN_PACK_SELECTION_ACTIVATION')
checks['QUALIFIER_UI_GATE']=has('scripts/qualify-v255-host.sh','PLUGIN_PACK_UI_TESTS')
failed=[name for name, ok in checks.items() if not ok]
for name, ok in checks.items(): print(f'{name}={"PASS" if ok else "FAIL"}')
if failed:
    print('OPENDECK_V255_PLUGIN_PACK_SELECTION_ACTIVATION=FAIL:'+','.join(failed))
    sys.exit(1)
print('OPENDECK_V255_PLUGIN_PACK_SELECTION_ACTIVATION=PASS')

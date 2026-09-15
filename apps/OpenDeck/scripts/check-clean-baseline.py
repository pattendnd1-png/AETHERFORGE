from pathlib import Path
import json, sys, tomllib, struct
root=Path(__file__).resolve().parents[1]
app=(root/'apps/opendeck-studio/src/App.tsx').read_text()
test=(root/'apps/opendeck-studio/src/App.test.tsx').read_text()
rust=(root/'apps/opendeck-studio/src-tauri/src/lib.rs').read_text()
css=(root/'apps/opendeck-studio/src/styles.css').read_text()
pkg=json.loads((root/'apps/opendeck-studio/package.json').read_text())
tsnode=json.loads((root/'apps/opendeck-studio/tsconfig.node.json').read_text())
cargo=tomllib.loads((root/'apps/opendeck-studio/src-tauri/Cargo.toml').read_text())
reqwest_dep=cargo.get('dependencies',{}).get('reqwest',{})
reqwest_features=set(reqwest_dep.get('features',[])) if isinstance(reqwest_dep,dict) else set()
icon_path=root/'apps/opendeck-studio/src-tauri/icons/icon.png'
icon_ok=False
if icon_path.is_file():
    data=icon_path.read_bytes()
    if len(data) >= 33 and data[:8] == b'\x89PNG\r\n\x1a\n' and data[12:16] == b'IHDR':
        width,height,bit_depth,color_type,compression,filter_method,interlace=struct.unpack('>IIBBBBB', data[16:29])
        icon_ok=(width > 0 and height > 0 and bit_depth == 8 and color_type == 6 and compression == 0 and filter_method == 0)

generated_debris=[]
for p in root.rglob('*'):
    rel=p.relative_to(root).as_posix()
    parts=set(p.relative_to(root).parts)
    if p.is_dir() and parts.intersection({'node_modules','dist','target','__pycache__'}):
        generated_debris.append(rel)
        continue
    if p.is_file() and (
        p.suffix in {'.pyc','.pyo'}
        or p.name.endswith('.tsbuildinfo')
        or rel in {
            'apps/opendeck-studio/vite.config.js',
            'apps/opendeck-studio/vite.config.d.ts',
        }
    ):
        generated_debris.append(rel)

checks={
 'VERSION': pkg['version']=='2.0.0',
 'SOURCE_HYGIENE': len(generated_debris)==0,
 'WINDOWS_HIERARCHY': all(x in app for x in ['action-list','device-stage','inspector','Property Inspector','Actions']),
 'PLUS_8_KEYS': 'Array.from({ length: 8 }' in app,
 'PLUS_4_DIALS': '[1,2,3,4].map' in app and 'data-testid="dial"' in app,
 'SAFE_STORAGE': all(x in app for x in ['function loadKeys()', 'function persistKeys(', 'window.localStorage', 'catch {']),
 'TEST_ISOLATION': all(x in test for x in ['vi.clearAllMocks()', 'getItem.mockRestore()', 'setItem.mockRestore()']) and 'vi.restoreAllMocks()' not in test,
 'TS_NODE_NOEMIT': tsnode.get('compilerOptions',{}).get('allowImportingTsExtensions') is True and tsnode.get('compilerOptions',{}).get('noEmit') is True,
 'REQWEST_FEATURES': isinstance(reqwest_dep,dict) and reqwest_dep.get('version')=='0.12' and reqwest_dep.get('default-features') is False and {'json','rustls-tls'}.issubset(reqwest_features) and 'form' not in reqwest_features,
 'TAURI_ICON': icon_ok,
 'CLIPPY_OPEN_EXTERNAL': '&& status.success()' in rust and 'if let Ok(status) = Command::new(program).args(args).status() {\n            if status.success() {' not in rust,
 'CLIPPY_TEST_MODULE_LAST': rust.find('pub fn run()') >= 0 and rust.find('#[cfg(test)]') > rust.find('pub fn run()'),
 'OBS_V5': all(x in rust for x in ['GetVersion','GetSceneList','SetCurrentProgramScene','ToggleStream','ToggleRecord','ToggleInputMute','"op": 6']),
 'TWITCH_DCF': all(x in rust for x in ['oauth2/device','urn:ietf:params:oauth:grant-type:device_code','oauth2/validate']),
 'MARKETPLACE': 'https://marketplace.elgato.com' in rust and 'scan_marketplace_downloads' in rust,
 'NO_OLD_DAEMON': 'opendeck-daemon' not in '\n'.join(p.as_posix() for p in root.rglob('*')),
 'NO_DRAGONGLASS': 'dragonglass' not in (app+css).lower(),
 'NO_AUTOSTART': 'systemctl' not in rust and 'autostart' not in rust.lower(),
}
bad=[k for k,v in checks.items() if not v]
for k,v in checks.items(): print(f'OPENDECK_CLEAN_{k}={"PASS" if v else "FAIL"}')
if generated_debris:
 print('OPENDECK_CLEAN_SOURCE_DEBRIS='+';'.join(sorted(generated_debris)))
if bad:
 print('OPENDECK_CLEAN_BASELINE_SOURCE=FAIL:'+','.join(bad)); sys.exit(1)
print('OPENDECK_CLEAN_BASELINE_SOURCE=PASS')

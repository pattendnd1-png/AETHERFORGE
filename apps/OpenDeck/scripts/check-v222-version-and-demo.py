from pathlib import Path
import json, sys, re
root = Path(__file__).resolve().parents[1]
errors=[]
package=json.loads((root/'apps/opendeck-studio/package.json').read_text())
if package.get('version')!='2.0.22': errors.append('package version')
cargo=(root/'apps/opendeck-studio/src-tauri/Cargo.toml').read_text()
if not re.search(r'^version = "2\.0\.22"$', cargo, re.M): errors.append('cargo version')
tauri=json.loads((root/'apps/opendeck-studio/src-tauri/tauri.conf.json').read_text())
if tauri.get('version')!='2.0.22': errors.append('tauri version')
demo=root/'apps/opendeck-studio/src/qualify/demoWorkspace.ts'
if not demo.exists(): errors.append('demo workspace missing')
else:
    text=demo.read_text()
    for label in ['Scene','Mic','Spotify','Browser','OBS','Twitch','Discord','System','Volume','Zoom','Brightness','Media']:
        if repr(label) not in text and f'"{label}"' not in text: errors.append(f'demo label {label}')
app=(root/'apps/opendeck-studio/src/app/App.tsx').read_text()
main=(root/'apps/opendeck-studio/src/main.tsx').read_text()
if not all(marker in app for marker in ['qualification?: QualificationContext','qualification.enabled','qualification.phase','createQualificationWorkspace']): errors.append('qualification mode branch')
if '<App qualification={qualification}' not in main or 'await bridge.qualificationContext()' not in main: errors.append('qualification bootstrap wiring')
if errors:
    print('OPENDECK_V222_VERSION_DEMO_CONTRACT=FAIL')
    for e in errors: print('MISSING='+e)
    sys.exit(1)
print('OPENDECK_V222_VERSION_DEMO_CONTRACT=PASS')

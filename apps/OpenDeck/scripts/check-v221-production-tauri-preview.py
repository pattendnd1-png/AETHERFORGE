from pathlib import Path
import sys
root=Path(__file__).resolve().parents[1]
q=root/'scripts/qualify-v221-host.sh'
text=q.read_text() if q.exists() else ''
checks={
 'QUALIFIER_EXISTS': q.exists(),
 'PRODUCTION_TAURI_PREVIEW': 'npm run tauri -- build --no-bundle' in text,
 'PREVIEW_GATE_NAME': 'PREVIEW_TAURI_BUILD' in text,
 'PLAIN_CARGO_PREVIEW_REJECTED': 'gate "PREVIEW_RELEASE_BUILD"' not in text,
 'TAURI_BEFORE_BUILD_PRESENT': 'beforeBuildCommand' in (root/'apps/opendeck-studio/src-tauri/tauri.conf.json').read_text(),
 'TAURI_FRONTEND_DIST_PRESENT': 'frontendDist' in (root/'apps/opendeck-studio/src-tauri/tauri.conf.json').read_text(),
 'NO_DEV_SERVER_IN_QUALIFIER': 'npm run dev' not in text and 'localhost:1420' not in text,
}
for k,v in checks.items(): print(f'OPENDECK_V221_PRODUCTION_TAURI_{k}={"PASS" if v else "FAIL"}')
failed=[k for k,v in checks.items() if not v]
if failed:
 print('OPENDECK_V221_PRODUCTION_TAURI_PREVIEW_CONTRACT=FAIL:'+','.join(failed))
 raise SystemExit(1)
print('OPENDECK_V221_PRODUCTION_TAURI_PREVIEW_CONTRACT=PASS')

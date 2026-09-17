from pathlib import Path
import sys
root = Path(__file__).resolve().parents[1]
main = (root/'apps/opendeck-studio/src/main.tsx').read_text()
app = (root/'apps/opendeck-studio/src/app/App.tsx').read_text()
qualifier = (root/'scripts/qualify-v221-host.sh').read_text() if (root/'scripts/qualify-v221-host.sh').exists() else ''
checks = {
    'DIRECT_QUALIFICATION_PROP': 'qualification?: QualificationContext' in app and 'qualification.enabled' in app and 'qualification.phase' in app,
    'NO_IMPORT_TIME_QUERY_DECISION': 'new URLSearchParams(window.location.search)' not in app and 'QUALIFICATION_MODE' not in app and 'QUALIFICATION_PHASE' not in app,
    'BOOT_SHELL_BEFORE_CONTEXT_AWAIT': 'OpenDeckBootShell' in main and main.find('root.render(<OpenDeckBootShell') < main.find('await bridge.qualificationContext()'),
    'DIRECT_CONTEXT_TO_APP': '<App qualification={qualification}' in main,
    'BOOT_ERROR_BOUNDARY': 'OpenDeckErrorBoundary' in main,
    'DIAGNOSTIC_PNG_ON_READY_FAILURE': 'OpenDeck-v2.0.21-BOOT-DIAGNOSTIC.png' in qualifier and 'PREVIEW_BOOT_DIAGNOSTIC=PASS' in qualifier,
    'V221_VISUAL_READY': 'OPENDECK_V221_PREVIEW_SCREENSHOT=PASS' in qualifier,
}
failed = [name for name, ok in checks.items() if not ok]
for name, ok in checks.items():
    print(f'OPENDECK_V221_BOOTSTRAP_{name}={"PASS" if ok else "FAIL"}')
if failed:
    print('OPENDECK_V221_BOOTSTRAP_CONTRACT=FAIL:' + ','.join(failed))
    sys.exit(1)
print('OPENDECK_V221_BOOTSTRAP_CONTRACT=PASS')

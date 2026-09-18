#!/usr/bin/env python3
from pathlib import Path
import re

root = Path(__file__).resolve().parents[1]
# Before the version cut, this intentionally inspects the current activator.
activator_path = root / 'scripts/activate-v237-qualified.sh'
if not activator_path.exists():
    print('ACTIVATOR_PRESENT=FAIL')
    raise SystemExit(1)
text = activator_path.read_text()

checks = {}
for var in ('tmp_desktop', 'tmp_legacy'):
    match = re.search(rf'^{var}="\$DESKTOP_DIR/([^\n"]+)"', text, re.M)
    value = match.group(1) if match else ''
    checks[f'{var.upper()}_ENDS_DOT_DESKTOP'] = bool(value) and value.endswith('.desktop')

# The final canonical targets must remain .desktop files too.
checks['CANONICAL_DESKTOP_TARGET'] = 'DESKTOP_FILE="$DESKTOP_DIR/opendeck-studio.desktop"' in text
checks['CANONICAL_LEGACY_TARGET'] = 'LEGACY_DESKTOP_FILE="$DESKTOP_DIR/opendeck.desktop"' in text

for name, ok in checks.items():
    print(f'{name}={"PASS" if ok else "FAIL"}')
if not all(checks.values()):
    raise SystemExit(1)
print('OPENDECK_V237_DESKTOP_STAGE_EXTENSION=PASS')

#!/usr/bin/env python3
from pathlib import Path
import json, re

root = Path(__file__).resolve().parents[1]
required = [
    root / 'scripts/stage-v244-system-wide.sh',
    root / 'scripts/activate-v244-system-wide.sh',
    root / 'scripts/rollback-v244-system-wide.sh',
    root / 'scripts/uninstall-v244-system-wide.sh',
    root / 'scripts/check-v244-system-menu-launch.sh',
]
checks = {}
for path in required:
    checks[f'FILE_{path.name}'] = path.exists()

cargo = (root / 'Cargo.toml').read_text()
checks['WORKSPACE_VERSION_2_0_44'] = 'version = "2.0.44"' in cargo
conf = json.loads((root / 'apps/opendeck-studio/src-tauri/tauri.conf.json').read_text())
checks['TAURI_VERSION_2_0_44'] = conf.get('version') == '2.0.44'

if required[0].exists():
    stage = required[0].read_text()
    checks['STAGE_OPT_VERSION_ROOT'] = '/opt/opendeck-plus' in stage and 'TARGET_VERSION="2.0.44"' in stage and 'INSTALL_ROOT="$OPT_ROOT/$TARGET_VERSION"' in stage
    checks['STAGE_MANIFEST'] = 'INSTALL-MANIFEST.txt' in stage and 'SHA256SUMS.txt' in stage
    checks['STAGE_UDEV'] = '70-opendeck-streamdeck.rules' in stage
else:
    checks['STAGE_OPT_VERSION_ROOT'] = False
    checks['STAGE_MANIFEST'] = False
    checks['STAGE_UDEV'] = False

if required[1].exists():
    activate = required[1].read_text()
    checks['ACTIVATE_CURRENT_LINK'] = '/opt/opendeck-plus/current' in activate
    checks['ACTIVATE_SYSTEM_BIN'] = '/usr/local/bin' in activate and 'SYSTEM_BIN="$LOCAL_BIN_DIR/opendeck-studio"' in activate
    checks['ACTIVATE_SYSTEM_DESKTOP'] = '/usr/share/applications' in activate and 'SYSTEM_DESKTOP="$SYSTEM_APP_DIR/opendeck-studio.desktop"' in activate
    checks['ACTIVATE_REMOVE_USER_SHADOWS_LAST'] = 'REMOVE_USER_SHADOWS_AFTER_SYSTEM_SWITCH' in activate
else:
    checks['ACTIVATE_CURRENT_LINK'] = False
    checks['ACTIVATE_SYSTEM_BIN'] = False
    checks['ACTIVATE_SYSTEM_DESKTOP'] = False
    checks['ACTIVATE_REMOVE_USER_SHADOWS_LAST'] = False

if required[4].exists():
    smoke = required[4].read_text()
    checks['MENU_SMOKE_SYSTEM_DESKTOP'] = '/usr/share/applications' in smoke and 'TEMP_SYSTEM_FILE="$SYSTEM_APP_DIR/$DESKTOP_ID.desktop"' in smoke
    checks['MENU_SMOKE_INSTALLED_BINARY'] = '/opt/opendeck-plus/2.0.44/bin/opendeck-studio' in smoke
else:
    checks['MENU_SMOKE_SYSTEM_DESKTOP'] = False
    checks['MENU_SMOKE_INSTALLED_BINARY'] = False

for name, ok in checks.items():
    print(f'{name}={"PASS" if ok else "FAIL"}')
if not all(checks.values()):
    raise SystemExit(1)
print('OPENDECK_V244_SYSTEM_WIDE_INSTALL_CONTRACT=PASS')

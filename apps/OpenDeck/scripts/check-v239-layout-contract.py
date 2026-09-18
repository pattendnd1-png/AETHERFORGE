#!/usr/bin/env python3
from pathlib import Path

root = Path(__file__).resolve().parents[1]
css = (root / 'apps/opendeck-studio/src/styles.css').read_text()
checks = {
    'device_stage_row': '.device-stage { grid-row:1;',
    'page_navigator_row': '.page-navigator { grid-row:2;',
    'inspector_handle_row': '.inspector-resize-handle { grid-row:3;',
    'configuration_strip_row': '.configuration-strip { grid-row:4;',
}
failed = []
for name, needle in checks.items():
    if needle not in css:
        failed.append(name)
if failed:
    print('OPENDECK_V239_LAYOUT_CONTRACT=FAIL')
    for name in failed:
        print(f'MISSING={name}')
    raise SystemExit(1)
print('OPENDECK_V239_LAYOUT_CONTRACT=PASS')

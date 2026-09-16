#!/usr/bin/env python3
from pathlib import Path
import sys
root = Path(__file__).resolve().parents[1]
app_test = (root / 'apps/opendeck-studio/src/App.test.tsx').read_text()
qualifier = (root / 'scripts/qualify-v212-host.sh').read_text()
checks = {
    'DIAL_TEST_USES_PREVIOUS_PAGE_ACTION': "name: 'Previous Page'" in app_test,
    'DIAL_TEST_DOES_NOT_NAVIGATE_PREVIOUS_CONTROL': "name: 'Previous control'" not in app_test,
    'QUALIFIER_USES_V212_RELEASE_LABEL': 'OPENDECK_V2_0_12_RENDER_PERFORMANCE' in qualifier,
    'QUALIFIER_HAS_NO_STALE_V210_RELEASE_LABEL': 'OPENDECK_V2_0_10_RENDER_PERFORMANCE' not in qualifier,
    'QUALIFIER_TARGETS_V212_SCREENSHOT': 'OpenDeck-v2.0.12-QUALIFICATION.png' in qualifier,
}
for key, ok in checks.items(): print(f'OPENDECK_V212_FINAL_CLOSURE_{key}={"PASS" if ok else "FAIL"}')
if not all(checks.values()): sys.exit(1)
print('OPENDECK_V212_FINAL_CLOSURE=PASS')

#!/usr/bin/env python3
from pathlib import Path
import sys
root = Path(__file__).resolve().parents[1]
app_test = (root / 'apps/opendeck-studio/src/App.test.tsx').read_text()
qualifier = (root / 'scripts/qualify-v219-host.sh').read_text()
checks = {
    'DIAL_TEST_USES_PREVIOUS_PAGE_ACTION': "name: 'Previous Page'" in app_test,
    'DIAL_TEST_DOES_NOT_NAVIGATE_PREVIOUS_CONTROL': "name: 'Previous control'" not in app_test,
    'QUALIFIER_USES_V219_RELEASE_LABEL': 'OPENDECK_V2_0_19_RENDER_PERFORMANCE' in qualifier,
    'QUALIFIER_HAS_NO_STALE_RELEASE_LABELS': all(x not in qualifier for x in ['OPENDECK_V2_0_10_RENDER_PERFORMANCE','OPENDECK_V2_0_11_RENDER_PERFORMANCE','OPENDECK_V2_0_12_RENDER_PERFORMANCE','OPENDECK_V2_0_13_RENDER_PERFORMANCE','OPENDECK_V2_0_15_RENDER_PERFORMANCE','OPENDECK_V2_0_16_RENDER_PERFORMANCE']),
    'QUALIFIER_TARGETS_V219_SCREENSHOT': 'OpenDeck-v2.0.19-QUALIFICATION.png' in qualifier,
    'PRODUCTION_TAURI_PREVIEW': 'gate "PREVIEW_TAURI_BUILD"' in qualifier and 'npm run tauri -- build --no-bundle' in qualifier,
    'VISUAL_PARITY_CONTRACT': 'check-v219-visual-parity-closure.py' in qualifier,
    'SCREENSHOT_BEFORE_TESTS_CONTRACT': 'check-v219-screenshot-before-tests.py' in qualifier,
    'DIAL_STACKS_DEFERRED_TO_V219': 'OPENDECK_DIAL_STACKS=DEFERRED_TO_2.0.20' in qualifier,
    'CANDIDATE_BOUND_SCREENSHOT_CONTRACT': 'check-v219-candidate-bound-screenshot.py' in qualifier,
}
for key, ok in checks.items(): print(f'OPENDECK_V219_FINAL_CLOSURE_{key}={"PASS" if ok else "FAIL"}')
if not all(checks.values()): sys.exit(1)
print('OPENDECK_V219_FINAL_CLOSURE=PASS')

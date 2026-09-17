#!/usr/bin/env python3
from pathlib import Path
import sys

root = Path(__file__).resolve().parents[1]
app_test = (root / 'apps/opendeck-studio/src/App.test.tsx').read_text()
qualifier = (root / 'scripts/qualify-v218-host.sh').read_text()

single_page_negative = "expect(screen.queryByRole('button', { name: 'Page 1' })).not.toBeInTheDocument();" in app_test
multi_page_positive = "expect(screen.getByRole('button', { name: 'Page 1' })).toHaveAttribute('aria-pressed', 'true');" in app_test

markers = {
    'FRONTEND_BUILD': 'gate "FRONTEND_BUILD"',
    'PREVIEW_TAURI_BUILD': 'gate "PREVIEW_TAURI_BUILD"',
    'PREVIEW_SCREENSHOT_PASS': 'OPENDECK_V218_PREVIEW_SCREENSHOT=PASS:',
    'FRONTEND_TESTS': 'gate "FRONTEND_TESTS"',
    'FRONTEND_LINT': 'gate "FRONTEND_LINT"',
}
pos = {name: qualifier.find(marker) for name, marker in markers.items()}
all_present = all(value >= 0 for value in pos.values())
order_ok = all_present and (
    pos['FRONTEND_BUILD']
    < pos['PREVIEW_TAURI_BUILD']
    < pos['PREVIEW_SCREENSHOT_PASS']
    < pos['FRONTEND_TESTS']
    < pos['FRONTEND_LINT']
)

checks = {
    'SINGLE_PAGE_STRIP_HIDDEN_TEST': single_page_negative,
    'MULTI_PAGE_NAV_TEST_PRESERVED': multi_page_positive,
    'SCREENSHOT_PRECEDES_FRONTEND_TESTS': order_ok,
}
for key, ok in checks.items():
    print(f'OPENDECK_V218_{key}={"PASS" if ok else "FAIL"}')

bad = [key for key, ok in checks.items() if not ok]
if bad:
    print('OPENDECK_V218_SCREENSHOT_BEFORE_TESTS_CONTRACT=FAIL:' + ','.join(bad))
    sys.exit(1)
print('OPENDECK_V218_SCREENSHOT_BEFORE_TESTS_CONTRACT=PASS')

#!/usr/bin/env python3
from pathlib import Path
import sys

root = Path(__file__).resolve().parents[1]
test_path = root / 'apps/opendeck-studio/src/components/PropertyInspector.test.tsx'
demo_path = root / 'apps/opendeck-studio/src/qualify/demoWorkspace.ts'
test = test_path.read_text()
demo = demo_path.read_text()

start = test.find("it('renders the canonical interaction rail for a non-stacked dial'")
end = test.find("it('shows stack cycling separately", start)
block = test[start:end] if start >= 0 and end > start else ''

checks = {
    'test_block_found': bool(block),
    'plain_dial_is_dial_3': 'page.slots.dials[2]' in block,
    'plain_dial_heading': "Configure: Dial 3" in block,
    'legacy_action_wheel_dial_not_used': 'page.slots.dials[1]' not in block,
    'qualification_action_wheel_still_dial_2': 'page.slots.dials[1].actionWheel = {' in demo,
    'qualification_stack_still_dial_1': 'page.slots.dials[0].dialStack = {' in demo,
}
failed = [name for name, ok in checks.items() if not ok]
if failed:
    for name in failed:
        print(f'FRONTEND_TEST_CLOSURE_ERROR={name}')
    print('OPENDECK_V245_FRONTEND_TEST_CLOSURE=FAIL')
    sys.exit(1)
print('OPENDECK_V245_FRONTEND_TEST_CLOSURE=PASS')

#!/usr/bin/env python3
from pathlib import Path
import sys

root = Path(__file__).resolve().parents[1]
key = (root / 'apps/opendeck-studio/src/components/KeyControl.tsx').read_text()
test = (root / 'apps/opendeck-studio/src/components/DeviceEditor.test.tsx').read_text()
errors=[]
if "name: 'Key 1: Scene'" not in test:
    errors.append('missing regression assertion for contextual key accessible name')
if 'aria-label={`Key ${slot.position + 1}: ${title}`}' not in key:
    errors.append('KeyControl does not expose contextual key accessible name')
if 'aria-label={title}' in key:
    errors.append('legacy ambiguous key aria-label remains')
if errors:
    for e in errors: print(f'OPENDECK_V227_ACCESSIBILITY_NAME_CONTRACT=FAIL:{e}')
    sys.exit(1)
print('OPENDECK_V227_ACCESSIBILITY_NAME_CONTRACT=PASS')

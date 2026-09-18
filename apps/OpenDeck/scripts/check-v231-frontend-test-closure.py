#!/usr/bin/env python3
from pathlib import Path
import sys

root = Path(__file__).resolve().parents[1]
property_test = (root / 'apps/opendeck-studio/src/components/PropertyInspector.test.tsx').read_text()
stack_test = (root / 'apps/opendeck-studio/src/components/DialStackEditor.test.tsx').read_text()
app_test = (root / 'apps/opendeck-studio/src/App.test.tsx').read_text()

checks = {
    'release_version': ('Cargo.toml', 'version = "2.0.31"'),
    'studio_version': ('apps/opendeck-studio/package.json', '"version": "2.0.31"'),
    'tauri_version': ('apps/opendeck-studio/src-tauri/Cargo.toml', 'version = "2.0.31"'),
    'property_non_stack_dial': ('apps/opendeck-studio/src/components/PropertyInspector.test.tsx', 'const slot = page.slots.dials[1];'),
    'property_stack_cycle_coverage': ('apps/opendeck-studio/src/components/PropertyInspector.test.tsx', "expect(screen.getByText('Cycles stack entry')).toBeInTheDocument();"),
    'property_stack_callbacks': ('apps/opendeck-studio/src/components/PropertyInspector.test.tsx', 'onDialStackRemoveStack: vi.fn(),'),
    'stack_label_value_1': ('apps/opendeck-studio/src/components/DialStackEditor.test.tsx', "getByLabelText('Stack entry 1 label')"),
    'stack_label_value_2': ('apps/opendeck-studio/src/components/DialStackEditor.test.tsx', "getByLabelText('Stack entry 2 label')"),
    'app_release': ('apps/opendeck-studio/src/App.test.tsx', "describe('OpenDeck 2.0.31 editor'"),
}
errors=[]
for name,(rel,needle) in checks.items():
    path=root/rel
    text=path.read_text(errors='replace') if path.exists() else ''
    if needle not in text:
        errors.append(f'{name}:missing:{needle}')

# Regressions that caused the v2.0.30 host FRONTEND_TESTS failure must stay gone.
forbidden = {
    'stack_label_getbytext_volume': "getByText('Volume')",
    'stack_label_getbytext_obs': "getByText('OBS Studio')",
    'property_qualification_stack_as_nonstack': 'const slot=page.slots.dials[0]',
}
for name, needle in forbidden.items():
    haystack = stack_test if name.startswith('stack_') else property_test
    if needle in haystack:
        errors.append(f'{name}:forbidden:{needle}')

if '2.0.30' in app_test:
    errors.append('app_test_stale_version:2.0.30')

if errors:
    print('OPENDECK_V231_FRONTEND_TEST_CLOSURE=FAIL')
    for error in errors:
        print('FRONTEND_TEST_CLOSURE_ERROR=' + error)
    sys.exit(1)
print('OPENDECK_V231_FRONTEND_TEST_CLOSURE=PASS')

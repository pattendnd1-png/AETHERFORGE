#!/usr/bin/env python3
from pathlib import Path
import sys
root=Path(__file__).resolve().parents[1]
app=(root/'apps/opendeck-studio/src/App.test.tsx').read_text()
prop=(root/'apps/opendeck-studio/src/components/PropertyInspector.test.tsx').read_text()
checks={
 'DIAL_ASSIGNMENT_USES_PREVIOUS_PAGE_ACTION': "name: 'Previous Page'" in app,
 'DIAL_ASSIGNMENT_DOES_NOT_USE_PREVIOUS_CONTROL_NAV': "name: 'Previous control'" not in app,
 'SAVE_STATUS_ACCESSIBILITY': "getByLabelText('Save status: Saved')" in app,
 'DIAL_INTERACTION_ACCESSIBLE_NAMES': 'Press Single press action' in prop and 'Rotate Left Counter-clockwise' in prop,
}
for k,v in checks.items(): print(f'OPENDECK_V213_TEST_CONTRACT_{k}={"PASS" if v else "FAIL"}')
if not all(checks.values()):
 print('OPENDECK_V213_TEST_CONTRACT=FAIL'); sys.exit(1)
print('OPENDECK_V213_TEST_CONTRACT=PASS')

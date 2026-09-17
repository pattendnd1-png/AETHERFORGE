#!/usr/bin/env python3
from pathlib import Path
p = Path('apps/opendeck-studio/src/components/PropertyInspector.tsx')
s = p.read_text()
needle = 'className="touch-strip-mode-editor" aria-label="Touch strip mode"'
assert needle not in s, 'duplicate accessible name: wrapper and select both expose "Touch strip mode"'
assert '<select aria-label="Touch strip mode"' in s, 'touch-strip mode select must remain explicitly labelled'
print('OPENDECK_V222_TOUCH_STRIP_A11Y_NAME=PASS')

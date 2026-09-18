#!/usr/bin/env python3
from pathlib import Path
import sys
root = Path(__file__).resolve().parents[1]
app = (root/'apps/opendeck-studio/src/app/App.tsx').read_text()
css = (root/'apps/opendeck-studio/src/styles.css').read_text()
checks = {
    'NO_CANONICAL_CANVAS_CONSTANTS': 'CANONICAL_CANVAS_WIDTH' not in app and 'CANONICAL_CANVAS_HEIGHT' not in app,
    'NO_FIT_SCALE_STATE': 'fitScale' not in app and '--opendeck-fit-scale' not in app,
    'NO_CANONICAL_CANVAS_WRAPPER': 'canonical-canvas' not in app and '.canonical-canvas' not in css,
    'VIEWPORT_FILLED': '.opendeck-viewport { position:relative; width:100%; height:100%;' in css,
    'SHELL_FILLED': '.app-shell { width:100%; height:100%; min-width:0; min-height:0;' in css,
    'RESPONSIVE_CHROME': '@media (max-width:1200px)' in css and '--sidebar-width:158px;' in css and '--action-width:260px;' in css,
    'COMPACT_HEIGHT': '@media (max-height:820px)' in css,
}
failed = [name for name, ok in checks.items() if not ok]
for name, ok in checks.items():
    print(f'{name}={"PASS" if ok else "FAIL"}')
if failed:
    print('OPENDECK_V246_FLUID_WINDOW_FILL_CLOSURE=FAIL:' + ','.join(failed))
    sys.exit(1)
print('OPENDECK_V246_FLUID_WINDOW_FILL_CLOSURE=PASS')

#!/usr/bin/env python3
from pathlib import Path
import re

root = Path(__file__).resolve().parents[1]
app = (root/'apps/opendeck-studio/src/app/App.tsx').read_text()
main = (root/'apps/opendeck-studio/src/main.tsx').read_text()
bridge = (root/'apps/opendeck-studio/src/bridge.ts').read_text()
css = (root/'apps/opendeck-studio/src/styles.css').read_text()

required_app = [
    'CANONICAL_CANVAS_WIDTH = 1536',
    'CANONICAL_CANVAS_HEIGHT = 1024',
    'Math.min(',
    'window.innerWidth / CANONICAL_CANVAS_WIDTH',
    'window.innerHeight / CANONICAL_CANVAS_HEIGHT',
    "qualificationMode ? 1 : fitScale",
    'className="opendeck-viewport"',
    'className="canonical-canvas"',
]
for marker in required_app:
    if marker not in app:
        raise SystemExit(f'OPENDECK_V225_CANONICAL_FIT=FAIL:APP_MISSING:{marker}')

for marker in [
    'appMaximize: () => getCurrentWindow().maximize()',
]:
    if marker not in bridge:
        raise SystemExit(f'OPENDECK_V225_CANONICAL_FIT=FAIL:BRIDGE_MISSING:{marker}')

if "if (!qualification.enabled)" not in main or 'bridge.appMaximize()' not in main:
    raise SystemExit('OPENDECK_V225_CANONICAL_FIT=FAIL:NORMAL_MODE_MAXIMIZE_MISSING')

required_css = [
    '.opendeck-viewport',
    '.canonical-canvas',
    'width:1536px',
    'height:1024px',
    'scale(var(--opendeck-fit-scale))',
]
for marker in required_css:
    if marker not in css:
        raise SystemExit(f'OPENDECK_V225_CANONICAL_FIT=FAIL:CSS_MISSING:{marker}')

if re.search(r'@media\s*\(max-width:\s*(?:1380|1180)px\)', css):
    raise SystemExit('OPENDECK_V225_CANONICAL_FIT=FAIL:LEGACY_REFLOW_MEDIA_QUERY_PRESENT')

if 'min-width:1100px' in css or 'min-height:720px' in css:
    raise SystemExit('OPENDECK_V225_CANONICAL_FIT=FAIL:ROOT_MINIMUM_BLOCKS_SMALL_VIEWPORT')


tauri = (root/'apps/opendeck-studio/src-tauri/tauri.conf.json').read_text()
if '"minWidth": 1100' in tauri or '"minHeight": 720' in tauri:
    raise SystemExit('OPENDECK_V225_CANONICAL_FIT=FAIL:TAURI_LEGACY_MINIMUM_BLOCKS_SMALL_DISPLAY')
for marker in ['"minWidth": 640', '"minHeight": 480']:
    if marker not in tauri:
        raise SystemExit(f'OPENDECK_V225_CANONICAL_FIT=FAIL:TAURI_WINDOW_FIT_MISSING:{marker}')

# Qualification mode must stay 1:1 for pixel parity.
if 'qualificationMode ? 1 : fitScale' not in app:
    raise SystemExit('OPENDECK_V225_CANONICAL_FIT=FAIL:QUALIFICATION_NOT_LOCKED_1X')

print('OPENDECK_V225_CANONICAL_FIT=PASS')

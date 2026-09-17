from pathlib import Path
import sys
p = Path('apps/opendeck-studio/src-tauri/src/streamdeck/runtime.rs')
s = p.read_text()
bad = 'StreamDeckInputEvent::DialRotate { index: 0, ticks: 2, pressed: false },'
if bad in s:
    print('OPENDECK_V208_RUSTFMT_REGRESSION=FAIL:INLINE_DIALROTATE_LITERAL')
    sys.exit(1)
print('OPENDECK_V208_RUSTFMT_REGRESSION=PASS')

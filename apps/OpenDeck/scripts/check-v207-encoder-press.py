from pathlib import Path
import sys

root = Path(__file__).resolve().parents[1]
types = (root / 'apps/opendeck-studio/src/types.ts').read_text()
workspace = (root / 'apps/opendeck-studio/src/model/workspace.ts').read_text()
hardware = (root / 'apps/opendeck-studio/src/app/hardware-events.ts').read_text()
inspector = (root / 'apps/opendeck-studio/src/inspector/ActionInspector.tsx').read_text()
runtime = (root / 'apps/opendeck-studio/src-tauri/src/streamdeck/runtime.rs').read_text()
ts_test = (root / 'apps/opendeck-studio/src/app/hardware-events.test.ts').read_text()

checks = {
    'TS_EVENT_CARRIES_PRESSED_STATE': "{ kind: 'dialRotate'; index: number; ticks: number; pressed: boolean }" in types,
    'INTERACTIONS_HAVE_PRESSED_ROTATION': "'pressRotateLeft'" in workspace and "'pressRotateRight'" in workspace,
    'RESOLVER_MAPS_PRESSED_LEFT': "event.pressed ? 'pressRotateLeft' : 'rotateLeft'" in hardware,
    'RESOLVER_MAPS_PRESSED_RIGHT': "event.pressed ? 'pressRotateRight' : 'rotateRight'" in hardware,
    'INSPECTOR_EXPOSES_PRESSED_ROTATION': "'pressRotateLeft'" in inspector and "'pressRotateRight'" in inspector,
    'RUNTIME_EVENT_CARRIES_PRESSED': 'pressed: bool' in runtime,
    'RUNTIME_USES_HELD_DIAL_STATE': 'pressed: self.dials[index]' in runtime,
    'TS_REGRESSION_TEST_PRESENT': "distinguishes pressed dial rotation from ordinary rotation" in ts_test,
    'RUST_REGRESSION_TEST_PRESENT': "marks_rotation_as_pressed_when_encoder_button_is_held" in runtime,
}
for key, passed in checks.items():
    print(f'OPENDECK_V207_ENCODER_{key}={"PASS" if passed else "FAIL"}')
bad = [key for key, passed in checks.items() if not passed]
if bad:
    print('OPENDECK_V207_ENCODER_PRESS_CONTRACT=FAIL:' + ','.join(bad))
    sys.exit(1)
print('OPENDECK_V207_ENCODER_PRESS_CONTRACT=PASS')

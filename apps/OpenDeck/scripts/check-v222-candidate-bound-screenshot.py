#!/usr/bin/env python3
from pathlib import Path

root = Path(__file__).resolve().parents[1]
bridge = (root/'apps/opendeck-studio/src/bridge.ts').read_text()
harness = (root/'apps/opendeck-studio/src/perf/QualificationHarness.tsx').read_text()
qual = (root/'apps/opendeck-studio/src-tauri/src/qualification.rs').read_text()
lib = (root/'apps/opendeck-studio/src-tauri/src/lib.rs').read_text()
host = (root/'scripts/qualify-v222-host.sh').read_text() if (root/'scripts/qualify-v222-host.sh').exists() else ''
capture = (root/'scripts/capture-v222-window.sh').read_text() if (root/'scripts/capture-v222-window.sh').exists() else ''

required = {
    'bridge focus command': "qualificationFocusWindow" in bridge,
    'harness awaits focus before visual metrics': 'await bridge.qualificationFocusWindow()' in harness and harness.index('await bridge.qualificationFocusWindow()') < harness.index("qualificationRecordUiMetrics({ kind: 'visual'"),
    'rust focus command': 'pub fn qualification_focus_window' in qual and '.set_focus()' in qual,
    'rust focus ack': 'OpenDeck-v2.0.22-FOCUS-ACK.json' in qual and '"release": "2.0.22"' in qual,
    'command registered': 'qualification::qualification_focus_window' in lib,
    'host waits focus ack': 'FOCUS_ACK=' in host and 'wait_focus_ack' in host,
    'host validates visual release': 'validate_visual_release' in host and '2.0.22' in host,
    'capture accepts candidate pid': 'EXPECTED_PID=' in capture and 'EXPECTED_RELEASE=' in capture,
    'capture verifies active pid when possible': 'getactivewindow' in capture and 'getwindowpid' in capture,
    'capture does not blindly call active window': 'spectacle -b -n -a' in capture and 'activate_candidate_window' in capture,
}
missing = [name for name, ok in required.items() if not ok]
if missing:
    raise SystemExit('OPENDECK_V222_CANDIDATE_BOUND_SCREENSHOT=FAIL:' + ','.join(missing))
print('OPENDECK_V222_CANDIDATE_BOUND_SCREENSHOT=PASS')

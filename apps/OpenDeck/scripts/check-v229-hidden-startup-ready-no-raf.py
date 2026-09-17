#!/usr/bin/env python3
from pathlib import Path

root = Path(__file__).resolve().parents[1]
harness = (root / 'apps/opendeck-studio/src/perf/QualificationHarness.tsx').read_text()
startup = harness.index("if (phase === 'startup')")
focus = harness.index('const focusAck = await bridge.qualificationFocusWindow();')
block = harness[startup:focus]
checks = {
    'STARTUP_BRANCH_PRESENT': "if (phase === 'startup')" in block,
    'NO_RAF_SETTLE_IN_STARTUP': 'settleFrames(' not in block,
    'NO_REQUEST_ANIMATION_FRAME_IN_STARTUP': 'requestAnimationFrame' not in block,
    'NO_FOCUS_IN_STARTUP': 'qualificationFocusWindow' not in block,
    'DIRECT_STARTUP_RECORD': "kind: 'startup'" in block,
    'REACT_READY_MILESTONE': 'const reactReadyMs = elapsed();' in block,
    'WORKSPACE_READY_MILESTONE': 'workspaceReadyMs: reactReadyMs' in block,
    'DIRECT_READY_MILESTONE': 'const visualReadyMs = elapsed();' in block,
}
for name, ok in checks.items():
    print(f'{name}={"PASS" if ok else "FAIL"}')
if not all(checks.values()):
    raise SystemExit(1)
print('OPENDECK_V229_HIDDEN_STARTUP_READY_NO_RAF=PASS')

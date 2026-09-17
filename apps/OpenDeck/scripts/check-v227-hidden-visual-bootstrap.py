#!/usr/bin/env python3
from pathlib import Path
p=Path(__file__).resolve().parents[1]/'apps/opendeck-studio/src/perf/QualificationHarness.tsx'
s=p.read_text()
startup=s.index("if (phase === 'startup')")
focus=s.index('const focusAck = await bridge.qualificationFocusWindow();')
frames=s.index('await settleFrames(4);')
# Startup branch must occur before focus and return before visual path.
assert startup < focus, 'startup branch must precede visible qualification path'
# Critical regression: visible qualification must show/focus before any rAF settling.
assert focus < frames, 'visual/performance must focus/show before settleFrames(4)'
# Startup remains hidden: no focus call inside startup block.
startup_block=s[startup:focus]
assert 'qualificationFocusWindow' not in startup_block
print('OPENDECK_V227_HIDDEN_VISUAL_BOOTSTRAP=PASS')

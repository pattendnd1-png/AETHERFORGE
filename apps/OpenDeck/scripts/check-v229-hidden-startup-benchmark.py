#!/usr/bin/env python3
from pathlib import Path
import json

root = Path(__file__).resolve().parents[1]
qualifier = (root / 'scripts/qualify-v229-host.sh').read_text()
harness = (root / 'apps/opendeck-studio/src/perf/QualificationHarness.tsx').read_text()
bridge = (root / 'apps/opendeck-studio/src/bridge.ts').read_text()
main = (root / 'apps/opendeck-studio/src/main.tsx').read_text()
qrs = (root / 'apps/opendeck-studio/src-tauri/src/qualification.rs').read_text()
config = json.loads((root / 'apps/opendeck-studio/src-tauri/tauri.conf.json').read_text())
window = config['app']['windows'][0]

checks = {
    'PHASE_STARTUP': "'visual' | 'performance' | 'startup'" in bridge,
    'WINDOW_INITIAL_HIDDEN': window.get('visible') is False,
    'NORMAL_SHOW': 'appShow' in bridge and 'bridge.appShow()' in main,
    'VISUAL_SHOWS_EXPLICITLY': '.show()' in qrs and 'qualification_focus_window' in qrs,
    'STARTUP_BRANCH': "phase === 'startup'" in harness,
    'STARTUP_RECORD': "kind: 'startup'" in harness,
    'STARTUP_NO_FOCUS_CONTRACT': "if (phase === 'startup')" in harness and "qualificationFocusWindow" in harness,
    'STARTUP_JSON': 'STARTUP_JSON=' in qualifier,
    'STARTUP_PHASE_LAUNCH': 'launch_candidate startup' in qualifier,
    'NO_VISUAL_STARTUP_LAUNCH': 'pid="$(launch_candidate visual)"; wait_file "$VISUAL_JSON" "$pid" 1500' not in qualifier,
    'WARM_SAMPLE_20': 'for run in {1..20}' in qualifier,
    'PRESERVE_EARLY_VISUAL': 'QUALIFIED_VISUAL_JSON' in qualifier,
    'NO_SECOND_VISUAL_RECAPTURE': 'QUALIFICATION-FINAL.png' not in qualifier,
    'IDLE_HIDDEN': 'OPENDECK_V229_IDLE_PHASE=startup' in qualifier,
}
for name, ok in checks.items():
    print(f'{name}={"PASS" if ok else "FAIL"}')
if not all(checks.values()):
    raise SystemExit(1)
print('OPENDECK_V229_HIDDEN_STARTUP_BENCHMARK_CONTRACT=PASS')

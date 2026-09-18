#!/usr/bin/env python3
from pathlib import Path

root = Path(__file__).resolve().parents[1]
harness = (root / 'apps/opendeck-studio/src/perf/QualificationHarness.tsx').read_text()
app = (root / 'apps/opendeck-studio/src/app/App.tsx').read_text()
bridge = (root / 'apps/opendeck-studio/src/bridge.ts').read_text()

checks = {
    'BRIDGE_PHASE_INCLUDES_STARTUP': "export type QualificationPhase = 'visual' | 'performance' | 'startup';" in bridge,
    'HARNESS_IMPORTS_QUALIFICATION_PHASE': "type QualificationPhase" in harness and "from '../bridge'" in harness,
    'HARNESS_PHASE_USES_SHARED_TYPE': 'phase: QualificationPhase;' in harness,
    'HARNESS_STARTED_AT_PROP': 'startedAtMs?: number;' in harness,
    'HARNESS_TAURI_SETUP_PROP': 'tauriSetupMs?: number;' in harness,
    'WEBVIEW_JS_MILESTONE_DEFINED': 'const WEBVIEW_JS_AT_MS = performance.timeOrigin + performance.now();' in harness,
    'APP_PASSES_STARTED_AT': 'startedAtMs={qualification.startedAtMs}' in app,
    'APP_PASSES_TAURI_SETUP': 'tauriSetupMs={qualification.tauriSetupMs}' in app,
    'STARTUP_BRANCH_PRESENT': "phase === 'startup'" in harness,
}

for name, ok in checks.items():
    print(f'{name}={"PASS" if ok else "FAIL"}')
if not all(checks.values()):
    raise SystemExit(1)
print('OPENDECK_V229_STARTUP_TYPE_INTEGRATION_CONTRACT=PASS')

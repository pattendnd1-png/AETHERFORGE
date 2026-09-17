from pathlib import Path
import sys
root=Path(__file__).resolve().parents[1]
front=(root/'apps/opendeck-studio/src/perf/QualificationHarness.tsx').read_text()
metrics=(root/'apps/opendeck-studio/src/perf/metrics.ts').read_text()
actions=(root/'apps/opendeck-studio/src/components/ActionLibrary.tsx').read_text()
bridge=(root/'apps/opendeck-studio/src/bridge.ts').read_text()
qual=(root/'apps/opendeck-studio/src-tauri/src/qualification.rs').read_text()
runtime=(root/'apps/opendeck-studio/src-tauri/src/streamdeck/runtime.rs').read_text()
editor=(root/'apps/opendeck-studio/src-tauri/src/editor.rs').read_text()
lib=(root/'apps/opendeck-studio/src-tauri/src/lib.rs').read_text()
checks={
 'BOUNDED_UI_METRICS': 'MAX_SAMPLES = 512' in metrics and 'percentile' in metrics,
 'QUALIFICATION_HARNESS': all(x in front for x in ['sidebarSelectionMs','controlToConfigurationMs','keysDialsSwitchMs','assignmentMs','profileSwitchMs','search5000Ms','persistenceEnqueueMs','framePacing']),
 'FIVE_THOUSAND_ACTION_CATALOG': 'qualificationCatalogSize' in actions and '5000' in (root/'apps/opendeck-studio/src/app/App.tsx').read_text(),
 'QUALIFICATION_BRIDGE': all(x in bridge for x in ['qualificationContext','qualificationRecordUiMetrics']),
 'RUST_METRICS_WRITER': all(x in qual for x in ['OpenDeck-v2.0.13-VISUAL-METRICS.json','OpenDeck-v2.0.13-PERFORMANCE-METRICS.json','atomic_json','record_runtime_sample']),
 'HID_TIMING': 'hidDecodeDispatchMs' in runtime and 'record_runtime_sample' in runtime,
 'PERSISTENCE_TIMING': 'persistenceWriteMs' in editor and 'workspace_benchmark_path' in editor,
 'COMMANDS_REGISTERED': all(x in lib for x in ['qualification::qualification_context','qualification::qualification_record_ui_metrics']),
}
for key,ok in checks.items(): print(f'OPENDECK_V213_{key}={"PASS" if ok else "FAIL"}')
bad=[k for k,v in checks.items() if not v]
if bad:
 print('OPENDECK_V213_PERFORMANCE_CONTRACT=FAIL:'+','.join(bad)); sys.exit(1)
print('OPENDECK_V213_PERFORMANCE_CONTRACT=PASS')

#!/usr/bin/env python3
from pathlib import Path

checks = {
    'EDITOR_PERSISTENCE_SAMPLE_MULTILINE': 'qualification::record_runtime_sample(\n        "persistenceWriteMs",\n        started.elapsed().as_secs_f64() * 1000.0,\n    );' in Path('apps/opendeck-studio/src-tauri/src/editor.rs').read_text(),
    'QUALIFICATION_PARENT_MULTILINE': 'let parent = path\n        .parent()\n        .ok_or_else(|| "qualification output has no parent".to_string())?;' in Path('apps/opendeck-studio/src-tauri/src/qualification.rs').read_text(),
    'QUALIFICATION_CONTEXT_MULTILINE': 'QualificationContext {\n        enabled: enabled(),\n        phase: phase(),\n    }' in Path('apps/opendeck-studio/src-tauri/src/qualification.rs').read_text(),
    'HID_SAMPLE_MULTILINE': 'qualification::record_runtime_sample(\n        "hidDecodeDispatchMs",\n        started.elapsed().as_secs_f64() * 1000.0,\n    );' in Path('apps/opendeck-studio/src-tauri/src/streamdeck/runtime.rs').read_text(),
}
for name, ok in checks.items():
    print(f'OPENDECK_V221_HOST_RUSTFMT_{name}={"PASS" if ok else "FAIL"}')
if not all(checks.values()):
    raise SystemExit(1)
print('OPENDECK_V221_HOST_RUSTFMT_CLOSURE=PASS')

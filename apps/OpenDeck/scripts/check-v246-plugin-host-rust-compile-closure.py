#!/usr/bin/env python3
from pathlib import Path
import sys
root=Path(__file__).resolve().parents[1]
path=root/'apps/opendeck-studio/src-tauri/src/plugin_host/mod.rs'
text=path.read_text()
checks={
    'NO_UNUSED_READ_IMPORT': 'io::{Read, Write}' not in text and 'io::Write' in text,
    'FEEDBACK_EVENT_CLONE': '#[derive(Clone, Debug, Serialize)]\n#[serde(rename_all = "camelCase")]\nstruct PluginFeedbackEvent' in text,
    'ZIP_ENCLOSED_NAME_PATHBUF_API': 'let Some(name) = file.enclosed_name() else { continue; };' in text,
    'NO_OBSOLETE_PATH_MAP': '.enclosed_name().map(Path::to_path_buf)' not in text,
    'TAURI_EMIT_FEEDBACK_PRESENT': 'app.emit(PLUGIN_FEEDBACK_EVENT, feedback)' in text,
}
for name, ok in checks.items():
    print(f'{name}={"PASS" if ok else "FAIL"}')
failed=[name for name,ok in checks.items() if not ok]
if failed:
    print('OPENDECK_V246_PLUGIN_HOST_RUST_COMPILE_CLOSURE=FAIL:'+','.join(failed))
    sys.exit(1)
print('OPENDECK_V246_PLUGIN_HOST_RUST_COMPILE_CLOSURE=PASS')

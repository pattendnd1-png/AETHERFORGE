#!/usr/bin/env python3
from pathlib import Path

p = Path('apps/opendeck-studio/src-tauri/src/qualification.rs')
text = p.read_text()
checks = {
    'SET_TITLE_RUSTFMT_MULTILINE': '''    window\n        .set_title(&title)\n        .map_err(|error| error.to_string())?;''' in text,
    'FOCUS_ACK_ATOMIC_JSON_SINGLE_LINE': '    atomic_json(&output_dir()?.join("OpenDeck-v2.0.21-FOCUS-ACK.json"), &ack)?;' in text,
}
for name, ok in checks.items():
    print(f'OPENDECK_V220_RUSTFMT_HOST_DIFF_{name}={"PASS" if ok else "FAIL"}')
if not all(checks.values()):
    raise SystemExit(1)
print('OPENDECK_V220_RUSTFMT_HOST_DIFF=PASS')

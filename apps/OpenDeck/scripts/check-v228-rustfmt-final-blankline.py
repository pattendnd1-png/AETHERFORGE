#!/usr/bin/env python3
from pathlib import Path

path = Path('apps/opendeck-studio/src-tauri/src/qualification.rs')
text = path.read_text()
needle = '        .unwrap_or_else(|| "visual".into())\n}\n\nfn epoch_ms() -> f64 {'
stale = '        .unwrap_or_else(|| "visual".into())\n}\n\n\nfn epoch_ms() -> f64 {'
checks = {
    'EPOCH_FN_HAS_SINGLE_SEPARATOR_BLANK_LINE': needle in text,
    'HOST_REPORTED_DOUBLE_BLANK_REMOVED': stale not in text,
}
for name, ok in checks.items():
    print(f'OPENDECK_V228_RUSTFMT_FINAL_{name}={"PASS" if ok else "FAIL"}')
if not all(checks.values()):
    print('OPENDECK_V228_RUSTFMT_FINAL=FAIL')
    raise SystemExit(1)
print('OPENDECK_V228_RUSTFMT_FINAL=PASS')

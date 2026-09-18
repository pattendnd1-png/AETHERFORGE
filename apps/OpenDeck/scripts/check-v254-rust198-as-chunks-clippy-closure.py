#!/usr/bin/env python3
from pathlib import Path
import sys
root=Path(__file__).resolve().parents[1]
paths=[
    root/'apps/opendeck-studio/src-tauri/src/pack_manager.rs',
    root/'apps/opendeck-studio/src-tauri/src/plugin_host/mod.rs',
]
checks={}
for path in paths:
    text=path.read_text()
    key=path.name if path.name != 'mod.rs' else 'plugin_host_mod.rs'
    checks[f'{key}:AS_CHUNKS_2'] = '.as_chunks::<2>().0.iter()' in text
    checks[f'{key}:NO_CHUNKS_EXACT_2'] = '.chunks_exact(2)' not in text
    checks[f'{key}:EVEN_LENGTH_GUARD'] = 'is_multiple_of(2)' in text
for name, ok in checks.items():
    print(f'{name}={"PASS" if ok else "FAIL"}')
failed=[name for name,ok in checks.items() if not ok]
if failed:
    print('OPENDECK_V254_RUST198_AS_CHUNKS_CLIPPY_CLOSURE=FAIL:'+','.join(failed))
    sys.exit(1)
print('OPENDECK_V254_RUST198_AS_CHUNKS_CLIPPY_CLOSURE=PASS')

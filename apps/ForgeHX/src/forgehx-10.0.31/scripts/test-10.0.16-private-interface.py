#!/usr/bin/env python3
from pathlib import Path
import re

src = Path('crates/forgehx-dsp/src/direct_pipewire.rs').read_text()
runtime = Path('crates/forgehx-dsp/src/runtime.rs').read_text()

if not re.search(r'pub\(crate\) enum RuntimeControl\b', runtime):
    raise SystemExit('FAIL: RuntimeControl is not crate-private as expected')

m = re.search(r'pub struct DirectRuntimeShared\s*\{(?P<body>.*?)\n\}', src, re.S)
if not m:
    m = re.search(r'pub\(crate\) struct DirectRuntimeShared\s*\{(?P<body>.*?)\n\}', src, re.S)
if not m:
    raise SystemExit('FAIL: DirectRuntimeShared struct not found')
body = m.group('body')

if re.search(r'^\s*pub control:\s*Arc<Mutex<RuntimeControl>>', body, re.M):
    raise SystemExit('FAIL: public DirectRuntimeShared::control exposes crate-private RuntimeControl')
if not re.search(r'^\s*pub\(crate\) control:\s*Arc<Mutex<RuntimeControl>>', body, re.M):
    raise SystemExit('FAIL: DirectRuntimeShared::control is not explicitly crate-private')

print('PASS: ForgeHX 10.0.16 private-interface visibility contract')

#!/usr/bin/env python3
from pathlib import Path
root = Path(__file__).resolve().parents[1]
checks = {
    'haste uses is_multiple_of': 'dpi % 100 != 0' not in (root/'crates/forgehx-mouse/src/protocol/haste_v1.rs').read_text(),
    'saga uses is_multiple_of': 'dpi % 50 != 0' not in (root/'crates/forgehx-mouse/src/protocol/saga_pro.rs').read_text(),
    'openrgb removes redundant u16 clamp': '.min(65535)' not in (root/'crates/forgehx-backends/src/openrgb.rs').read_text(),
    'ratbag removes trim before split_whitespace': 'rest.trim().split_whitespace()' not in (root/'crates/forgehx-backends/src/ratbag.rs').read_text(),
    'ratbag unit struct avoids default construction': 'RatbagClient::default()' not in (root/'crates/forgehx-backends/src/services.rs').read_text(),
}
failed=[name for name, ok in checks.items() if not ok]
if failed:
    for name in failed: print(f'FAIL: {name}')
    raise SystemExit(1)
print('PASS: ForgeHX 10.0.23 clippy cleanup contract')

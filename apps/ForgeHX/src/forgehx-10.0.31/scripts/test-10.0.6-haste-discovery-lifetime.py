#!/usr/bin/env python3
from pathlib import Path
src = Path('crates/forgehx-mouse/src/protocol/haste_v1.rs').read_text()
needle = 'pub fn discover_config_interface(expected_product_id: u16) -> Option<String> {'
start = src.index(needle)
end = src.index('\n}\n\npub fn select_config_interface', start) + 2
body = src[start:end]
failed = []
if 'let selected_path =' not in body:
    failed.append('discovery result is not bound to a local before api drops')
if not body.rstrip().endswith('selected_path\n}'):
    failed.append('function does not return the local selected_path value')
if 'api.device_list()\n        .find' in body and '.map(|device| device.path().to_string_lossy().into_owned())\n}' in body:
    failed.append('tail-expression iterator still borrows HidApi through block teardown')
if failed:
    raise SystemExit('ForgeHX 10.0.6 Haste discovery lifetime regression failed: ' + ', '.join(failed))
print('PASS: ForgeHX 10.0.6 Haste discovery lifetime regression')

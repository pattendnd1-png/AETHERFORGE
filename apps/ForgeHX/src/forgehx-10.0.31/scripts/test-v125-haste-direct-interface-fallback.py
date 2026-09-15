from pathlib import Path
p=Path('crates/forgehx-mouse/src/protocol/haste_v1.rs')
s=p.read_text()
need=[
    'discover_config_interface',
    'select_config_interface(device)',
    '.or_else(|| discover_config_interface(device.product_id))',
    'device.interface_number() == CONFIG_INTERFACE',
    'usage_page == CONFIG_USAGE_PAGE',
    'device.vendor_id() == HYPERX_VID',
    'matches!(device.product_id(), HASTE_WIRELESS_PID | HASTE_WIRED_PID)',
]
missing=[x for x in need if x not in s]
if missing:
    raise SystemExit('FAIL: missing direct Haste HID interface fallback markers: '+repr(missing))
print('PASS: ForgeHX 10.0.6 direct Haste HID interface fallback contract')

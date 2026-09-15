#!/usr/bin/env python3
from pathlib import Path
root=Path(__file__).resolve().parents[1]
gui=root/'crates/forgehx-gui/src'
hw=gui/'hardware.rs'
assert hw.exists(), 'hardware.rs missing'
s=hw.read_text(); dp=(gui/'device_page.rs').read_text(); main=(gui/'main.rs').read_text()
for marker in ['HardwareControl', 'Battery', 'capability_owners', 'SetVolume', 'SetMute', 'Backend']:
    assert marker in s, f'missing hardware marker {marker}'
for cls in ['Headset','Controller','Webcam','Mousepad','Monitor','AudioInterface','UsbAudio']:
    assert f'DeviceClass::{cls}' in dp, f'Hardware tab missing class {cls}'
assert 'DeviceTab::Hardware' in dp and 'Self::Hardware => "Hardware"' in dp
assert 'mod hardware;' in main
for forbidden in ['hid_write', 'write_feature', 'send_feature_report', 'raw_hid', 'vendor_write']:
    assert forbidden not in s.lower(), f'raw writer forbidden in hardware settings: {forbidden}'
print('ForgeHX 10.0.6 HyperX hardware settings invariants passed.')

#!/usr/bin/env python3
from pathlib import Path
root = Path(__file__).resolve().parents[1]
gui = root/'crates/forgehx-gui/src'
app = (gui/'app.rs').read_text()
nav = (gui/'nav.rs').read_text()
home = (gui/'home.rs').read_text()
settings = (gui/'settings.rs').read_text()
main = (gui/'main.rs').read_text()
joined = '\n'.join([app, nav, home, settings, main])
for forbidden in ['mod all_devices;', 'Page::AllDevices', 'ListAllDevices', 'all_devices: Vec<DeviceInfo>', 'filters: all_devices::DeviceFilters']:
    assert forbidden not in joined, f'forbidden GUI universal inventory marker: {forbidden}'
assert 'self.hyperx_devices.iter().find' in app, 'selected() must resolve against hyperx_devices'
assert 'All detected devices' not in settings
assert 'watching all USB, HID, and audio devices' not in home
assert not (gui/'all_devices.rs').exists(), 'all_devices GUI module must be removed'
print('ForgeHX 10.0.6 HyperX-only GUI invariants passed.')

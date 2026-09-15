from pathlib import Path

root = Path(__file__).resolve().parents[1]

def read(path: str) -> str:
    return (root / path).read_text()

registry = read('crates/forgehx-mouse/src/registry.rs')
protocol = read('crates/forgehx-mouse/src/protocol/haste_v1.rs')
daemon = read('crates/forgehx-daemon/src/lib.rs')
udev = read('packaging/udev/70-forgehx.rules')
firmware = read('crates/forgehx-firmware/src/registry.rs')
gui = read('crates/forgehx-gui/src/mouse.rs')
core = read('crates/forgehx-core/src/lib.rs')
mic = read('crates/forgehx-mic/src/lib.rs')

# Exact user hardware identities.
assert '(0x03f0,0x028e)' in registry
assert '(0x03f0,0x048e)' in registry
assert '0x03f0, product_id: 0x0fbf' in firmware
assert 'ATTRS{idProduct}=="0fbf"' in udev

# Haste Wireless must be promoted to the complete native surface supported by
# the verified public protocol, rather than DPI/polling-only partial support.
for cap in ['Capability::Lighting', 'Capability::Dpi', 'Capability::PollingRate',
            'Capability::Bindings', 'Capability::BatteryStatus']:
    assert cap in registry, cap
assert 'complete:true' in registry.replace(' ', '')
assert 'wireless:true' in registry.replace(' ', '')
assert 'battery:true' in registry.replace(' ', '')

# Readback + native control primitives.
for marker in ['pub fn state(&self)', 'pub fn battery_status(&self)',
               'pub fn set_lighting(&self', 'pub fn set_button_assignment(&self',
               'pub fn set_lift_off_distance(&self', 'REPORT_DEVICE_INFO',
               'REPORT_DPI_SETTINGS', 'CMD_BUTTON_ASSIGNMENT']:
    assert marker in protocol, marker

# Daemon must expose the native operations and refresh battery telemetry.
for marker in ['mouse.state()', 'mouse.battery_status()', 'mouse.set_lighting(config)',
               'mouse.set_button_assignment(button, action)',
               'mouse.set_lift_off_distance(mm)']:
    assert marker in daemon, marker

# Lift-off distance is a first-class IPC/UI control.
assert 'SetLiftOffDistance' in core
assert 'LiftOffDistance' in gui

# SoloCast 2 must have exact inventory identity and a verified composite full-support contract, but no undocumented native writer.
assert 'SOLOCAST_2_IDS' in firmware
assert 'hyperx-solocast-2-firmware' in firmware
assert 'exact_hardware_match: true' in firmware
assert 'SOLOCAST_2_PRODUCT_ID: u16 = 0x0fbf' in mic
assert 'complete_microphone_support' in mic
assert 'solocast_2_exact_composite_stack_is_fully_supported' in daemon
assert 'pulsefire_haste_complete_driver_stays_full_with_compatibility_owners' in daemon

print('FORGEHX_V121_TARGET_HARDWARE=PASS')

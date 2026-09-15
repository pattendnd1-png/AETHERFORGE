from pathlib import Path
rule = Path('packaging/udev/70-forgehx.rules').read_text()
helper = Path('scripts/refresh-haste-hid-access.sh').read_text() if Path('scripts/refresh-haste-hid-access.sh').exists() else ''
install = Path('forgehx.install').read_text() if Path('forgehx.install').exists() else ''
pkgbuild = Path('PKGBUILD').read_text()
for pid in ('028e','048e'):
    vendor_product = f'ATTRS{{idVendor}}=="03f0", ATTRS{{idProduct}}=="{pid}"'
    lines = [line for line in rule.splitlines() if vendor_product in line and 'ID_USB_INTERFACE_NUM' in line]
    assert lines, f'missing exact interface-02 rule for {pid}'
    line = lines[0]
    assert 'ENV{ID_USB_INTERFACE_NUM}=="02"' in line, f'{pid} rule is not interface-02 scoped'
    assert 'MODE:="0666"' in line, f'{pid} interface-02 access is not deterministic'
    assert 'TAG+="uaccess"' in line, f'{pid} must retain uaccess tag'
assert 'ID_USB_INTERFACE_NUM=02' in helper
assert 'ID_VENDOR_ID=03f0' in helper
assert 'ID_MODEL_ID' in helper
assert 'chmod 0666' in helper
assert 'udevadm control --reload-rules' in helper
assert 'udevadm trigger' in helper
assert 'udevadm settle' in helper
assert 'refresh-haste-hid-access.sh' in install
assert 'install=forgehx.install' in pkgbuild
print('FORGEHX_10_0_5_HASTE_HID_ACCESS=PASS')

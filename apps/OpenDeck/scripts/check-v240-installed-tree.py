#!/usr/bin/env python3
import hashlib, os, sys
from pathlib import Path

mode = sys.argv[1] if len(sys.argv) > 1 else 'staged'
expected_sha = sys.argv[2] if len(sys.argv) > 2 else ''
target_home = Path(sys.argv[3]) if len(sys.argv) > 3 else Path.home()

opt_root = Path(os.environ.get('OPENDECK_OPT_ROOT', '/opt/opendeck-plus'))
local_bin = Path(os.environ.get('OPENDECK_LOCAL_BIN_DIR', '/usr/local/bin'))
app_dir = Path(os.environ.get('OPENDECK_SYSTEM_APP_DIR', '/usr/share/applications'))
icon_dir = Path(os.environ.get('OPENDECK_SYSTEM_ICON_DIR', '/usr/share/icons/hicolor/64x64/apps'))
udev_dir = Path(os.environ.get('OPENDECK_UDEV_DIR', '/etc/udev/rules.d'))
sbin_dir = Path(os.environ.get('OPENDECK_LOCAL_SBIN_DIR', '/usr/local/sbin'))
root = opt_root / '2.0.40'

def sha(path: Path) -> str:
    h=hashlib.sha256()
    with path.open('rb') as f:
        for chunk in iter(lambda:f.read(1024*1024), b''):
            h.update(chunk)
    return h.hexdigest()

def check(name, ok):
    print(f'{name}={"PASS" if ok else "FAIL"}')
    return bool(ok)

results=[]
results.append(check('INSTALL_ROOT', root.is_dir()))
binary=root/'bin/opendeck-studio'
results.append(check('BINARY_PRESENT', binary.is_file() and os.access(binary, os.X_OK)))
results.append(check('BINARY_SHA', bool(expected_sha) and binary.is_file() and sha(binary)==expected_sha))
results.append(check('VERSION_FILE', (root/'VERSION').read_text().strip()=='2.0.40' if (root/'VERSION').is_file() else False))
results.append(check('INSTALL_MANIFEST', (root/'manifest/INSTALL-MANIFEST.txt').is_file()))
results.append(check('SHA_MANIFEST', (root/'manifest/SHA256SUMS.txt').is_file()))
results.append(check('DESKTOP_TEMPLATE', (root/'share/applications/opendeck-studio.desktop').is_file()))
results.append(check('ICON_PAYLOAD', (root/'share/icons/hicolor/64x64/apps/opendeck-studio.png').is_file()))
results.append(check('UDEV_PAYLOAD', (root/'share/udev/70-opendeck-streamdeck.rules').is_file()))
results.append(check('ROLLBACK_PAYLOAD', (root/'libexec/opendeck-rollback').is_file()))
results.append(check('UNINSTALL_PAYLOAD', (root/'libexec/opendeck-uninstall').is_file()))

if mode == 'active':
    current=opt_root/'current'
    results.append(check('CURRENT_LINK', current.is_symlink() and current.resolve()==root.resolve()))
    sysbin=local_bin/'opendeck-studio'
    results.append(check('SYSTEM_COMMAND', sysbin.is_symlink() and sysbin.resolve()==binary.resolve()))
    results.append(check('SYSTEM_DESKTOP', (app_dir/'opendeck-studio.desktop').is_file()))
    results.append(check('SYSTEM_ICON', (icon_dir/'opendeck-studio.png').is_file()))
    results.append(check('SYSTEM_UDEV', (udev_dir/'70-opendeck-streamdeck.rules').is_file()))
    results.append(check('SYSTEM_ROLLBACK', os.access(sbin_dir/'opendeck-rollback', os.X_OK)))
    results.append(check('SYSTEM_UNINSTALL', os.access(sbin_dir/'opendeck-uninstall', os.X_OK)))
    results.append(check('USER_BIN_SHADOW_REMOVED', not (target_home/'.local/bin/opendeck-studio').exists() and not (target_home/'.local/bin/opendeck-studio').is_symlink()))
    results.append(check('USER_DESKTOP_SHADOW_REMOVED', not (target_home/'.local/share/applications/opendeck-studio.desktop').exists()))

if not all(results):
    raise SystemExit(1)
print(f'OPENDECK_V240_INSTALLED_TREE_{mode.upper()}=PASS')

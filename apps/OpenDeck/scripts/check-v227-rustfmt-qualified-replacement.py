#!/usr/bin/env python3
from pathlib import Path
import re

root = Path(__file__).resolve().parents[1]
component = (root / 'apps/opendeck-studio/src/components/UnifiedTouchControl.tsx').read_text()
activate = root / 'scripts/activate-v227-qualified.sh'

checks = []
# Host lint failure must be removed without changing the public prop shape.
checks.append(('UNUSED_SLOT_BINDING_REMOVED', not re.search(r'function UnifiedTouchControl\(\{[\s\S]{0,300}?\n\s*slot,', component)))
checks.append(('SLOT_PROP_COMPATIBILITY_PRESERVED', 'slot: ControlSlot;' in component))
checks.append(('ACTIVATOR_EXISTS', activate.is_file()))
if activate.is_file():
    text = activate.read_text()
    for name, needle in [
        ('KEEP_ONE_ROLLBACK', 'ROLLBACK_KEEP_VERSION="2.0.8"'),
        ('INSTALL_V227', 'TARGET_VERSION="2.0.27"'),
        ('CANONICAL_USER_BIN', 'USER_BIN="$HOME/.local/bin"'),
        ('CANONICAL_DESKTOP', 'opendeck-studio.desktop'),
        ('CANONICAL_ICON', 'ICON_DIR="$HOME/.local/share/icons/hicolor/256x256/apps"'),
        ('CLEAN_OLD_INSTALLS', 'cleanup_old_installs'),
        ('NO_AUTOSTART_CREATE', '.config/autostart' not in text),
        ('NO_SYSTEMD_CREATE', '.config/systemd/user' not in text),
    ]:
        if isinstance(needle, bool):
            checks.append((name, needle))
        else:
            checks.append((name, needle in text))

failed = [name for name, ok in checks if not ok]
for name, ok in checks:
    print(f'{name}={"PASS" if ok else "FAIL"}')
if failed:
    print('OPENDECK_V227_RUSTFMT_QUALIFIED_REPLACEMENT=FAIL:' + ','.join(failed))
    raise SystemExit(1)
print('OPENDECK_V227_RUSTFMT_QUALIFIED_REPLACEMENT=PASS')

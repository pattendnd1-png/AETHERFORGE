#!/usr/bin/env python3
from pathlib import Path

root = Path(__file__).resolve().parents[1]
main = (root / 'apps/opendeck-studio/src/main.tsx').read_text()
conf = (root / 'apps/opendeck-studio/src-tauri/tauri.conf.json').read_text()
activator_path = root / 'scripts/activate-v251-system-wide.sh'
qualifier_path = root / 'scripts/qualify-v251-host.sh'
menu_path = root / 'scripts/check-v251-system-menu-launch.sh'
activator = activator_path.read_text() if activator_path.exists() else ''
stage = (root / 'scripts/stage-v251-system-wide.sh').read_text()
qualifier = qualifier_path.read_text() if qualifier_path.exists() else ''
menu = menu_path.read_text() if menu_path.exists() else ''

normal = main.index('if (!qualification.enabled)')
show = main.find('bridge.appShow()', normal)
raf = main.find('requestAnimationFrame', normal)
checks = {
    'NORMAL_STARTUP_SHOWS_BEFORE_RAF': show != -1 and (raf == -1 or show < raf),
    'NATIVE_NORMAL_STARTUP_SHOW': 'startup::prepare_main_window(app)' in (root / 'apps/opendeck-studio/src-tauri/src/lib.rs').read_text(),
    'QUALIFICATION_STILL_STARTS_HIDDEN': '"visible": false' in conf,
    'SYSTEM_ACTIVATOR_EXISTS': activator_path.exists(),
    'SYSTEM_QUALIFIER_EXISTS': qualifier_path.exists(),
    'CANONICAL_DESKTOP_CATEGORY': 'Categories=Utility;' in stage and 'Categories=Utility;AudioVideo;' not in stage,
    'USER_SHADOWS_REMOVED_AFTER_SWITCH': 'REMOVE_USER_SHADOWS_AFTER_SYSTEM_SWITCH' in activator,
    'KDE_CACHE_REFRESH': 'kbuildsycoca6' in activator or 'kbuildsycoca5' in activator,
    'SYSTEM_MENU_PREACTIVATION_GATE': 'SYSTEM_MENU_LAUNCH_STAGED' in qualifier,
    'SYSTEM_MENU_POSTACTIVATION_GATE': 'SYSTEM_MENU_LAUNCH_CANONICAL' in qualifier,
    'SYSTEM_MENU_USES_DESKTOP_ID': 'gtk-launch "$DESKTOP_ID"' in menu,
    'SYSTEM_MENU_VISIBILITY_CHECK': 'OPENDECK_STARTUP_PROBE_FILE' in menu and 'nativeVisible' in menu and 'frontendVisible' in menu,
    'FINAL_NOTE_SAYS_SYSTEM_INSTALL': 'System_wide_install_ready' in qualifier,
}
for name, ok in checks.items():
    print(f'{name}={"PASS" if ok else "FAIL"}')
if not all(checks.values()):
    raise SystemExit(1)
print('OPENDECK_V251_VISIBLE_STARTUP_KDE_LAUNCHER_CONTRACT=PASS')

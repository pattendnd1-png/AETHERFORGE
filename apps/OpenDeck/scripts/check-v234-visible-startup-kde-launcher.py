#!/usr/bin/env python3
from pathlib import Path

root = Path(__file__).resolve().parents[1]
main = (root / 'apps/opendeck-studio/src/main.tsx').read_text()
conf = (root / 'apps/opendeck-studio/src-tauri/tauri.conf.json').read_text()
activator_path = root / 'scripts/activate-v234-qualified.sh'
qualifier_path = root / 'scripts/qualify-v234-host.sh'
activator = activator_path.read_text() if activator_path.exists() else ''
qualifier = qualifier_path.read_text() if qualifier_path.exists() else ''
menu_path = root / 'scripts/check-v234-menu-launch.sh'
menu = menu_path.read_text() if menu_path.exists() else ''

normal = main.index('if (!qualification.enabled)')
show = main.find('bridge.appShow()', normal)
raf = main.find('requestAnimationFrame', normal)
checks = {
    'NORMAL_STARTUP_SHOWS_BEFORE_RAF': show != -1 and (raf == -1 or show < raf),
    'QUALIFICATION_STILL_STARTS_HIDDEN': '"visible": false' in conf,
    'V234_ACTIVATOR_EXISTS': activator_path.exists(),
    'V234_QUALIFIER_EXISTS': qualifier_path.exists(),
    'CANONICAL_DESKTOP_CATEGORY': 'Categories=Utility;' in activator and 'Categories=Utility;AudioVideo;' not in activator,
    'LEGACY_DESKTOP_ALIAS_HIDDEN': 'opendeck.desktop' in activator and 'NoDisplay=true' in activator,
    'KDE_CACHE_REFRESH': 'kbuildsycoca6' in activator or 'kbuildsycoca5' in activator,
    'DESKTOP_VALIDATED_BEFORE_SWITCH': activator.find('desktop-file-validate') != -1 and activator.find('for link_name in opendeck-studio opendeck') > activator.find('desktop-file-validate'),
    'MENU_LAUNCH_GATE': 'MENU_LAUNCH' in qualifier,
    'MENU_LAUNCH_BEFORE_ACTIVATION': qualifier.find('gate "MENU_LAUNCH"') != -1 and qualifier.find('gate "MENU_LAUNCH"') < qualifier.find('gate "ACTIVATION"'),
    'MENU_LAUNCH_USES_DESKTOP_ID': 'gtk-launch "$DESKTOP_ID"' in menu or 'kioclient' in menu,
    'MENU_LAUNCH_VISIBILITY_CHECK': 'kdotool' in menu or 'xdotool' in menu,
    'FINAL_NOTE_SAYS_LAUNCH': 'Launch_OpenDeck_when_ready_to_run_the_new_binary' in qualifier,
}
for name, ok in checks.items():
    print(f'{name}={"PASS" if ok else "FAIL"}')
if not all(checks.values()):
    raise SystemExit(1)
print('OPENDECK_V234_VISIBLE_STARTUP_KDE_LAUNCHER_CONTRACT=PASS')

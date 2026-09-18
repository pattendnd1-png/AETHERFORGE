from pathlib import Path
import sys

root = Path(__file__).resolve().parents[1]
src = root / 'apps/opendeck-studio/src'
app = (src / 'app/App.tsx').read_text()
topbar = (src / 'components/TopBar.tsx').read_text()
profile = (src / 'components/ProfileManager.tsx').read_text()
pages = (src / 'components/PageNavigator.tsx').read_text()
actions = (src / 'components/ActionLibrary.tsx').read_text()
inspector = (src / 'components/PropertyInspector.tsx').read_text()
device = (src / 'components/DeviceEditor.tsx').read_text()
css = (src / 'styles.css').read_text()
combined = '\n'.join([app, topbar, profile, pages, actions, inspector, device])

required = {
    'keys_tab': 'role="tab"' in actions and '>Keys<' in actions,
    'dials_tab': '>Dials<' in actions,
    'page_options': 'aria-label="Page options"' in pages,
    'page_number_buttons': 'aria-label={`Page ${i + 1}`}' in pages,
    'configuration_region': 'data-layout-region="configuration"' in inspector,
    'device_surface': 'className="device-surface"' in device,
    'device_selector': 'aria-label="Device"' in topbar,
    'profile_selector': 'aria-label="Profile"' in profile,
    'profile_options': 'aria-label="Profile options"' in profile,
    'save_state_header': 'saveStatus' in topbar,
    'editor_toast': 'className="editor-toast"' in app and 'role="status"' in app,
    'action_mode_owner': "useState<ActionLibraryMode>('keys')" in app,
    'keys_filter': "supportsControl(item.id, 'key')" in actions,
    'dials_filter': "supportsControl(item.id, 'dial');" in actions,
}

forbidden = {
    'old_statusbar': '<footer className="statusbar"' in app,
    'old_property_title': '>Property Inspector<' in combined,
    'old_deck_card': 'className="deck-plus"' in device,
    'old_status_pill_text': 'Stream Deck + ·' in topbar,
    'old_deck_card_css': '.deck-plus {' in css,
    'touch_promoted_to_dials': "supportsControl(item.id, 'dial') || supportsControl(item.id, 'touch')" in actions,
}

for key, passed in required.items():
    print(f'OPENDECK_V207_INTERFACE_{key.upper()}={"PASS" if passed else "FAIL"}')
for key, present in forbidden.items():
    print(f'OPENDECK_V207_INTERFACE_FORBID_{key.upper()}={"FAIL" if present else "PASS"}')

bad = [key for key, passed in required.items() if not passed]
bad += [key for key, present in forbidden.items() if present]
if bad:
    print('OPENDECK_V207_INTERFACE_CONTRACT=FAIL:' + ','.join(bad))
    sys.exit(1)
print('OPENDECK_V207_INTERFACE_CONTRACT=PASS')

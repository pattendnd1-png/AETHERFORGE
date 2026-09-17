#!/usr/bin/env python3
from pathlib import Path

root = Path(__file__).resolve().parents[1]
css = (root / 'apps/opendeck-studio/src/styles.css').read_text()
app = (root / 'apps/opendeck-studio/src/app/App.tsx').read_text()
actions = (root / 'apps/opendeck-studio/src/components/ActionLibrary.tsx').read_text()
demo = (root / 'apps/opendeck-studio/src/qualify/demoWorkspace.ts').read_text()
qualification = (root / 'apps/opendeck-studio/src-tauri/src/qualification.rs').read_text()

checks = {
    'CLIPPY_COLLAPSIBLE_IF_CLOSED': 'if kind == "performance"\n        && let Some(object) = body.as_object_mut()' in qualification,
    'DARK_NATIVE_SELECTS': 'appearance:none' in css.replace(' ', '') and 'color-scheme:dark' in css.replace(' ', ''),
    'ACTION_COLUMN_TARGET': '--action-width: 388px' in css and 'margin:8px18px20px0' in css.replace(' ', ''),
    'DEFAULT_ACTION_PANEL_TARGET': 'actionPanelWidth: 388' in (root / 'apps/opendeck-studio/src/model/workspace.ts').read_text(),
    'SIDEBAR_TARGET_INSET': 'margin:8px020px8px' in css.replace(' ', ''),
    'DEVICE_STAGE_CENTER_COMPENSATION': 'padding:18px16px4px32px' in css.replace(' ', ''),
    'KEY_TARGET_GEOMETRY': 'grid-template-columns:repeat(4,120px)' in css.replace(' ', '') and 'grid-template-rows:repeat(2,112px)' in css.replace(' ', '') and 'column-gap:28px' in css.replace(' ', '') and 'row-gap:9px' in css.replace(' ', ''),
    'TOUCH_BEZEL_TARGET_GEOMETRY': 'width:654px' in css.replace(' ', '') and 'height:92px' in css.replace(' ', '') and 'padding:9px10px' in css.replace(' ', ''),
    'DIAL_LABELS_HAVE_ROOM': '.streamdeck-dials { width:654px; height:122px; margin:14px auto 0;' in css and '.dial-control { height:118px;' in css,
    'SINGLE_PAGE_NAV_HIDDEN': '{profile.pages.length > 1 && <PageNavigator' in app,
    'EDITOR_AUTO_PAGE_ROW': 'grid-template-rows:minmax(0,1fr)auto0var(--inspector-height,326px)' in css.replace(' ', ''),
    'CONFIG_TARGET_HEIGHT': 'workspace.preferences.inspectorHeight = 326' in demo and 'margin:08px20px' in css.replace(' ', ''),
    'ACTION_VIEWPORT_FILLS_TARGET': 'viewportHeight={740}' in actions,
    'KEY_DEPTH_PASS': '.control.key-control::before' in css and 'radial-gradient' in css,
    'DIAL_DEPTH_PASS': 'repeating-conic-gradient' in css,
}

failed = [name for name, ok in checks.items() if not ok]
for name, ok in checks.items():
    print(f'OPENDECK_V223_VISUAL_PARITY_{name}={"PASS" if ok else "FAIL"}')
if failed:
    print('OPENDECK_V223_VISUAL_PARITY_CLOSURE=FAIL:' + ','.join(failed))
    raise SystemExit(1)
print('OPENDECK_V223_VISUAL_PARITY_CLOSURE=PASS')

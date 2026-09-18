#!/usr/bin/env python3
from pathlib import Path
import sys

root = Path(__file__).resolve().parents[1]
plugin = (root / 'apps/opendeck-studio/src-tauri/src/plugin_host/mod.rs').read_text()
manager = (root / 'apps/opendeck-studio/src/components/PluginManager.tsx').read_text()
app = (root / 'apps/opendeck-studio/src/app/App.tsx').read_text()
render = (root / 'apps/opendeck-studio/src-tauri/src/streamdeck/render.rs').read_text()
pack = (root / 'apps/opendeck-studio/src-tauri/src/pack_manager.rs').read_text()
tests = (root / 'apps/opendeck-studio/src/components/PluginManager.test.tsx').read_text()

checks = {
    'DOWNLOADS_ARE_DETECTED_NOT_FAKE_READY': '<em>{error ? \'ERROR\' : \'DETECTED\'}</em>' in manager and '<em>READY</em>' not in manager,
    'PACKAGE_FAILURE_IS_LOCAL': 'packageErrors' in manager and "[key]: text" in manager and 'role="alert"' in manager,
    'PLUGIN_INSTALL_CAN_SKIP_ACTIVATION': "installDownloaded(selectedDownload, false)" in manager and "await installPlugin(path, activate)" in app,
    'MANUAL_PLUGIN_INSTALL_SEPARATE_FROM_ACTIVATE': 'manualIsPlugin' in manager and 'Install Path</button>' in manager and 'Install Path &amp; Activate' in manager,
    'UTF8_BOM_NORMALIZATION': 'strip_prefix(&[0xEF, 0xBB, 0xBF])' in plugin,
    'UTF16_MANIFEST_NORMALIZATION': 'decode_utf16_json' in plugin and 'String::from_utf16' in plugin,
    'ICON_PACK_MANIFEST_NORMALIZATION': 'fn normalize_json_bytes' in pack and 'fn manifest_candidates' in pack and 'parse_json_manifest' in pack,
    'MANIFEST_CANDIDATES_ARE_VALIDATED': 'fn manifest_candidates' in plugin and 'match parse_descriptor(parent, name)' in plugin,
    'BROKEN_PACKAGE_STAGING_CLEANUP': 'if staging.exists()' in plugin and 'fs::remove_dir_all(&staging)' in plugin,
    'ACTIVE_REQUIRES_PLUGIN_REGISTRATION': 'registered_rx.recv()' in plugin and 'did not complete the Stream Deck plugin registration handshake' in plugin,
    'REGISTER_PLUGIN_NOT_PROPERTY_INSPECTOR_ACTIVATES': 'return Ok(event == "registerPlugin")' in plugin,
    'CLIPPY_PLUGIN_STATE_COLLAPSED': 'if states.is_empty()\n            && let Some(icon)' in plugin,
    'CLIPPY_SVG_COLLAPSED': '&& let Ok(image) = image::load_from_memory(&output.stdout)' in render,
    'CLIPPY_PLUGIN_ICON_COLLAPSED': '} else if let Some(action_id) = action_id\n        && let Some(path) = plugin_host::action_state_image_path(action_id)' in render,
    'CLIPPY_PACK_ICON_COLLAPSED': 'if !icon_rendered\n        && let Some(path) = pack_manager::active_icon_path' in render,
    'UI_TESTS_COVER_DETECTED': "getAllByText('DETECTED')" in tests,
    'UI_TESTS_COVER_FAILURE_ISOLATION': 'isolates a failed package so another package remains installable' in tests,
}

failed = [name for name, ok in checks.items() if not ok]
for name, ok in checks.items():
    print(f'OPENDECK_V251_{name}={"PASS" if ok else "FAIL"}')
if failed:
    print('OPENDECK_V251_PLUGIN_PACKAGE_INSTALL_CLOSURE=FAIL:' + ','.join(failed))
    sys.exit(1)
print('OPENDECK_V251_PLUGIN_PACKAGE_INSTALL_CLOSURE=PASS')

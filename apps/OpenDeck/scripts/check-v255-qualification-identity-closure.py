#!/usr/bin/env python3
from pathlib import Path
root = Path(__file__).resolve().parents[1]
qualification = (root/'apps/opendeck-studio/src-tauri/src/qualification.rs').read_text()
startup = (root/'apps/opendeck-studio/src-tauri/src/startup.rs').read_text()
harness = (root/'apps/opendeck-studio/src/perf/QualificationHarness.tsx').read_text()
topbar = (root/'apps/opendeck-studio/src/components/TopBar.tsx').read_text()
sidebar = (root/'apps/opendeck-studio/src/components/AppSidebar.tsx').read_text()
plugin_host = (root/'apps/opendeck-studio/src-tauri/src/plugin_host/mod.rs').read_text()
qualifier = (root/'scripts/qualify-v255-host.sh').read_text()
checks = {
    'QUALIFICATION_VISUAL_FILENAME': 'OpenDeck-v2.0.55-VISUAL-METRICS.json' in qualification,
    'QUALIFICATION_FOCUS_FILENAME': 'OpenDeck-v2.0.55-FOCUS-ACK.json' in qualification,
    'QUALIFICATION_RELEASE_ACK': '"release": "2.0.55"' in qualification,
    'QUALIFIER_WAITS_SAME_VISUAL_FILENAME': 'OpenDeck-v2.0.55-VISUAL-METRICS.json' in qualifier,
    'QUALIFIER_WAITS_SAME_FOCUS_FILENAME': 'OpenDeck-v2.0.55-FOCUS-ACK.json' in qualifier,
    'HARNESS_RELEASE_MATCH': "release: '2.0.55'" in harness and "focusAck.release !== '2.0.55'" in harness,
    'STARTUP_RELEASE_MATCH': 'release: "2.0.55"' in startup,
    'PLUGIN_HOST_PROTOCOL_MATCH': 'HOST_PROTOCOL_VERSION: &str = "2.0.55"' in plugin_host,
    'TOPBAR_BRAND_MATCH': '2.0.55' in topbar,
    'SIDEBAR_BRAND_MATCH': '2.0.55' in sidebar,
    'NO_STALE_QUALIFICATION_FILENAMES': 'OpenDeck-v2.0.53-VISUAL-METRICS.json' not in qualification and 'OpenDeck-v2.0.54-VISUAL-METRICS.json' not in qualification,
}
failed=[name for name, ok in checks.items() if not ok]
for name, ok in checks.items(): print(f'OPENDECK_V255_{name}={"PASS" if ok else "FAIL"}')
if failed:
    print('OPENDECK_V255_QUALIFICATION_IDENTITY_CLOSURE=FAIL:'+','.join(failed))
    raise SystemExit(1)
print('OPENDECK_V255_QUALIFICATION_IDENTITY_CLOSURE=PASS')

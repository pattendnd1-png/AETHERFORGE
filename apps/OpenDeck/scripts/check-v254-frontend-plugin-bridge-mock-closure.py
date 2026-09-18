#!/usr/bin/env python3
from pathlib import Path
import re, sys
root=Path(__file__).resolve().parents[1]
t=(root/'apps/opendeck-studio/src/App.test.tsx').read_text()
required=[
 'pluginList','pluginHostStatus','pluginInstall','pluginRemove','pluginSetEnabled','pluginSetActive','pluginRestart',
 'pluginDispatchAction','pluginDispatchLifecycle','pluginDispatchHostEvent','pluginImportBundledProfile','pluginPropertyInspectorSession'
]
errors=[]
for name in required:
    if not re.search(rf'\b{name}\s*:\s*vi\.fn\(\)', t): errors.append('mock:'+name)
    if not re.search(rf'bridge\.{name}\)', t): errors.append('setup:'+name)
if 'pluginDispatchHostEvent).mockResolvedValue(undefined)' not in t:
    errors.append('host_event_not_resolved')
if errors:
    print('OPENDECK_V254_FRONTEND_PLUGIN_BRIDGE_MOCK_CLOSURE=FAIL:'+';'.join(errors)); sys.exit(1)
print('OPENDECK_V254_FRONTEND_PLUGIN_BRIDGE_MOCK_CLOSURE=PASS')

#!/usr/bin/env python3
from pathlib import Path
import sys

p = Path(__file__).resolve().parents[1] / 'apps/opendeck-studio/src/components/PluginPropertyInspector.tsx'
s = p.read_text()
checks = {
    'stable_plugin_uuid_scalar': "const pluginUuid = parsed?.pluginUuid ?? '';" in s,
    'stable_action_uuid_scalar': "const actionUuid = parsed?.actionUuid ?? '';" in s,
    'effect_does_not_reference_parsed': 'bridge.pluginPropertyInspectorSession(parsed.pluginUuid, parsed.actionUuid, context)' not in s,
    'effect_uses_stable_scalars': 'bridge.pluginPropertyInspectorSession(pluginUuid, actionUuid, context)' in s,
    'stable_dependency_array': '}, [pluginUuid, actionUuid, context]);' in s,
}
missing = [k for k,v in checks.items() if not v]
if missing:
    print('OPENDECK_V241_PLUGIN_PI_HOOK_LINT_CLOSURE=FAIL:' + ','.join(missing))
    sys.exit(1)
print('OPENDECK_V241_PLUGIN_PI_HOOK_LINT_CLOSURE=PASS')

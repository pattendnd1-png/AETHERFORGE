#!/usr/bin/env python3
from pathlib import Path
import sys
root=Path(__file__).resolve().parents[1]
path=root/'apps/opendeck-studio/src-tauri/src/plugin_host/mod.rs'
text=path.read_text()
checks={
    'CODE_PATH_MAC_IS_USED': 'code_path_mac: Option<String>' in text and 'manifest.code_path_mac.clone()' in text,
    'MERGED_KEY_DIAL_DEFAULT_INTERACTION': 'kind == "key" || kind == "dial"' in text and 'else if kinds.iter().any(|kind| kind == "dial")' not in text,
    'COLLAPSED_SET_STATE': 'if event == "setState"\n                && let Some(value)' in text,
    'COLLAPSED_SET_IMAGE': 'if let Some(image) = payload.get("image").and_then(Value::as_str)\n                && let Some((asset_id, path, data_url))' in text,
    'COLLAPSED_MULTI_ACTION_DESIRED_STATE': 'if request.is_in_multi_action\n                && let Some(desired) = request.user_desired_state' in text,
    'SORT_BY_KEY': 'sort_by_key(|plugin| plugin.name.to_lowercase())' in text,
    'NO_OLD_SORT_BY': '.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()))' not in text,
}
for name,ok in checks.items():
    print(f'{name}={"PASS" if ok else "FAIL"}')
failed=[name for name,ok in checks.items() if not ok]
if failed:
    print('OPENDECK_V252_PLUGIN_HOST_STRICT_CLIPPY_CLOSURE=FAIL:'+','.join(failed))
    sys.exit(1)
print('OPENDECK_V252_PLUGIN_HOST_STRICT_CLIPPY_CLOSURE=PASS')

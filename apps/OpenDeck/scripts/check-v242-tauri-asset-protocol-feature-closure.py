#!/usr/bin/env python3
from pathlib import Path
import json, sys
root=Path(__file__).resolve().parents[1]
conf=json.loads((root/'apps/opendeck-studio/src-tauri/tauri.conf.json').read_text())
cargo=(root/'apps/opendeck-studio/src-tauri/Cargo.toml').read_text()
enabled=bool(conf.get('app',{}).get('security',{}).get('assetProtocol',{}).get('enable'))
feature='protocol-asset' in cargo
if enabled and not feature:
    print('OPENDECK_V242_TAURI_ASSET_PROTOCOL_FEATURE_CLOSURE=FAIL:assetProtocol_enabled_but_protocol_asset_feature_missing')
    sys.exit(1)
print('OPENDECK_V242_TAURI_ASSET_PROTOCOL_FEATURE_CLOSURE=PASS')

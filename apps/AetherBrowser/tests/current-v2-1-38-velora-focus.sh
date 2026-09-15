#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
PROVIDERS="$ROOT/crates/aether-stream-providers/src/lib.rs"
LIVE="$ROOT/crates/aether-engine-servo/src/live.rs"
SCRIPT="$ROOT/scripts/velora-runtime-test.sh"
grep -qF 'id: "velora".into()' "$PROVIDERS"
grep -qF 'kind: ProviderKind::Velora' "$PROVIDERS"
grep -qF 'auth: WebSession' "$PROVIDERS"
grep -qF '"velora" => Some("https://velora.tv/".to_owned())' "$LIVE"
grep -qF 'https://velora.tv/' "$SCRIPT"
grep -qF 'VELORA_RUNTIME_VERIFY=PASS' "$SCRIPT"
! grep -qE 'youtube|music\.youtube|twitch' "$SCRIPT"
printf '%s\n' 'AETHER_BROWSER_V2_1_47_VELORA_FOCUS=PASS'

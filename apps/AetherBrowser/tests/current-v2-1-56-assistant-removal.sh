#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$ROOT"
fail(){ echo "AETHER_BROWSER_V2_1_56_ASSISTANT_REMOVAL=FAIL:$1"; exit 1; }

for path in \
  crates/aether-ai-bridge \
  integration/aetherai-browser-service \
  scripts/build-aetherai-browser-service.sh \
  packaging/wrappers/aetherai-browser-service \
  packaging/systemd/aether-browser-aetherai.service; do
  [[ ! -e "$path" ]] || fail "residual-path:$path"
done

if grep -RniE --exclude='Cargo.lock' --exclude='CHANGELOG.md' --exclude='README.md' \
  --exclude='current-v2-1-56-assistant-removal.sh' --exclude='install-current-tree.sh' --exclude-dir=target --exclude-dir=docs/superpowers \
  'aetherai|aether-ai|aether_ai|aether://ai|openai_action|openai_key|AETHER_BROWSER_AETHERAI' \
  Cargo.toml crates scripts integration packaging tests 2>/dev/null; then
  fail residual-integration-reference
fi

migration_count=$(grep -Eic 'aetherai|aether-ai|aether_ai|aether://ai|openai_action|openai_key|AETHER_BROWSER_AETHERAI' scripts/install-current-tree.sh || true)
[[ "$migration_count" -eq 1 ]] || fail "unexpected-installer-retired-reference-count:$migration_count"
grep -qF 'systemctl --user disable --now aether-browser-aetherai.service >/dev/null 2>&1 || true' scripts/install-current-tree.sh || fail retired-service-cleanup-missing

echo 'AETHER_BROWSER_V2_1_56_ASSISTANT_REMOVAL=PASS'

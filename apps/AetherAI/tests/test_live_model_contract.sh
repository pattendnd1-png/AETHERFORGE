#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
cd "$ROOT"
grep -q 'llama-b10649-bin-ubuntu-vulkan-x64.tar.gz' resources/inference/runtime-manifest.json
grep -q 'fee6f25a7c5c87310e9f75dd4b0a325a5a04bcbd46160f8c1a784cb816d01ad5' resources/inference/runtime-manifest.json
grep -q 'llama-b10649-bin-win-vulkan-x64.zip' resources/inference/runtime-manifest.json
grep -q '922bbc6cff5880106d1f5cd2451ce19f3c5e657acaf8aadb828501a07e9939d4' resources/inference/runtime-manifest.json
grep -q 'b0638f08417a2d3c8652760462eb5407c6e30173cf9608ad0820757a281eea0e' resources/model-catalog.json
grep -q 'AETHERAI_MODEL_SERVICE=PASS' apps/aetherai-cli/src/main.rs
grep -q 'AETHERAI_TOKEN_STREAM=PASS' apps/aetherai-cli/src/main.rs
echo AETHERAI_LIVE_MODEL_CONTRACT=PASS

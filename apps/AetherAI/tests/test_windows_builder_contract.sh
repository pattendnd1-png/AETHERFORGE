#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
cd "$ROOT"
grep -q 'x86_64-pc-windows-gnu' scripts/build-windows-release.sh
grep -q 'rust-mingw' scripts/build-windows-release.sh
grep -q 'llama-b10649-bin-win-vulkan-x64.zip' scripts/acquire-live-assets-windows.ps1
grep -q '922bbc6cff5880106d1f5cd2451ce19f3c5e657acaf8aadb828501a07e9939d4' scripts/acquire-live-assets-windows.ps1
grep -q 'AETHERAI_WINDOWS_NATIVE_VERIFY=PENDING_WINDOWS_HOST' scripts/build-windows-release.sh
echo AETHERAI_WINDOWS_BUILDER_CONTRACT=PASS

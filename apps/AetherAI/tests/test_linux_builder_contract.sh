#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
plan=$($ROOT/scripts/build-linux-release.sh --plan)
grep -q '^AETHERAI_PLATFORM=linux-x86_64$' <<<"$plan"
grep -q '^AETHERAI_ARTIFACT=AetherAI-v0.1.9-linux-x86_64.tar.gz$' <<<"$plan"
! grep -qi windows <<<"$plan"
grep -q 'tools/rust/x86_64-unknown-linux-gnu' "$ROOT/scripts/verify-linux-self-contained.sh"
echo AETHERAI_LINUX_BUILDER_CONTRACT=PASS

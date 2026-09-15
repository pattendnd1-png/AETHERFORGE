#!/usr/bin/env bash
set -Eeuo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
echo 'NOTICE: RC3 clean-rebase runner is superseded by v7.0.7-stable.' >&2
exec "$ROOT/RUN-AETHERFORGE-V7.0.7-STABLE.sh" "${1:-all}"

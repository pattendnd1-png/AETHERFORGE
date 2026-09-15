#!/usr/bin/env bash
set -Eeuo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec "$ROOT/RUN-AETHERFORGE-V6.2.16-STABLE.sh" "${1:-all}"

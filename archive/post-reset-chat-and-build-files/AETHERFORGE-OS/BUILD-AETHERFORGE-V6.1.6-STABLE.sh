#!/usr/bin/env bash
set -Eeuo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec "$ROOT/RUN-AETHERFORGE-V6.1.6-STABLE.sh" "${1:-all}"

#!/usr/bin/env bash
set -Eeuo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec "$ROOT/RUN-AETHERFORGE-V7.0.2-MAIN.sh" "${1:-all}"

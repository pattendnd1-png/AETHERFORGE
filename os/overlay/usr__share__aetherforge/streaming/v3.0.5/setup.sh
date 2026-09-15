#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
resolution="${1:-1080p}"
case "$resolution" in
  1080p|1920x1080) launcher="$ROOT/launch-1080p.sh" ;;
  1440p|2560x1440) launcher="$ROOT/launch-1440p.sh" ;;
  *) echo "Usage: ./setup.sh [1080p|1440p]" >&2; exit 64 ;;
esac
bash "$ROOT/install.sh"
bash "$ROOT/diagnose.sh"
exec bash "$launcher"

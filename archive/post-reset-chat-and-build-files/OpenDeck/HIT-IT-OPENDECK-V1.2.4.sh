#!/usr/bin/env bash
set -euo pipefail
HERE="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
LOG="$HOME/Downloads/OpenDeck-v1.2.4-BUILD.log"
: > "$LOG"
exec > >(tee -a "$LOG") 2>&1
echo 'OPENDECK_V1_2_4=START'
echo "LOG=$LOG"
exec "$HERE/UPGRADE-OPENDECK-V1.2.3-TO-V1.2.4.sh" "$@"

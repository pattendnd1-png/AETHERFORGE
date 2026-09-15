#!/usr/bin/env bash
set -euo pipefail
HERE="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
cd "$HERE"
sha256sum -c OpenDeck-v1.3.0-WINDOWS-CLEAN-REBUILD-3-SOURCE.tar.xz.sha256
sha256sum -c HIT-IT-OPENDECK-v1.3.0-WINDOWS-CLEAN-REBUILD-3.sh.sha256
exec ./HIT-IT-OPENDECK-v1.3.0-WINDOWS-CLEAN-REBUILD-3.sh "$@"

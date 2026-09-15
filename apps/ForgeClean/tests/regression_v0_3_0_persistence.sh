#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
cd "$ROOT"
[[ -f forgeclean-organizer.service.in ]]
grep -Fq 'ExecStart=%h/.local/bin/forgeclean watch' forgeclean-organizer.service.in
grep -Fq 'Restart=always' forgeclean-organizer.service.in
grep -Fq 'WantedBy=default.target' forgeclean-organizer.service.in
grep -Fq 'systemctl --user enable forgeclean-organizer.service' install-local.sh
grep -Fq 'systemctl --user restart forgeclean-organizer.service' install-local.sh
grep -Fq 'loginctl enable-linger' install-local.sh
grep -Fq 'FORGECLEAN_PERSISTENT_SERVICE=PASS' install-local.sh
grep -Fq '"watch"' src/main.rs
grep -Fq 'FORGECLEAN_WATCH=ACTIVE' src/main.rs
echo FORGECLEAN_V0_3_0_PERSISTENCE=PASS

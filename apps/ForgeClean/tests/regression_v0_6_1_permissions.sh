#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
cd "$ROOT"

grep -Fq 'normalize_source_permissions()' hit-it-template.sh
grep -Fq 'chmod 0755 "$BUILD_DIR"' hit-it-template.sh
grep -Fq 'chmod -R u+rwX,go+rX "$BUILD_DIR"' hit-it-template.sh
grep -Fq 'run_stage FORGECLEAN_PERMISSION_NORMALIZE normalize_source_permissions' hit-it-template.sh
printf 'FORGECLEAN_V0_6_1_PERMISSIONS=PASS\n'

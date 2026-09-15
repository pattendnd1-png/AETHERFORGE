#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
SCRIPT="$ROOT/scripts/build-linux-release.sh"
plan=$("$SCRIPT" --plan)
grep -q '^AETHERAI_ARTIFACT=AetherAI-v0.1.9-linux-x86_64.tar.gz$' <<<"$plan"
grep -q 'VERIFY_RC=' "$SCRIPT"
grep -q 'AETHERAI_HOST_VERIFY_LOG' "$SCRIPT"
archive_line=$(grep -n 'tar -czf "\$ARCHIVE"' "$SCRIPT" | head -1 | cut -d: -f1)
exit_line=$(grep -n 'exit "\$VERIFY_RC"' "$SCRIPT" | tail -1 | cut -d: -f1)
[[ -n "$archive_line" && -n "$exit_line" && "$archive_line" -lt "$exit_line" ]]
echo AETHERAI_LINUX_ARCHIVE_ON_VERIFY_FAILURE_TEST=PASS

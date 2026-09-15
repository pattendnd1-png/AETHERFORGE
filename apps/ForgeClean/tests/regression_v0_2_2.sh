#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd -P)"
MAIN="$ROOT/src/main.rs"
if grep -Fq 'if let Ok(sudo_user) = env::var("SUDO_USER") {' "$MAIN"; then
  echo FORGECLEAN_V0_2_2_COLLAPSIBLE_IF=FAIL
  exit 1
fi
grep -Fq 'if let Ok(sudo_user) = env::var("SUDO_USER")' "$MAIN"
grep -Fq '&& !sudo_user.is_empty()' "$MAIN"
grep -Fq '&& sudo_user != "root"' "$MAIN"
grep -Fq '&& let Some(home) = passwd_home(&sudo_user)?' "$MAIN"
echo FORGECLEAN_V0_2_2_COLLAPSIBLE_IF=PASS
echo FORGECLEAN_V0_2_2_REGRESSION=PASS

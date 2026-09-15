#!/usr/bin/env bash
set -u -o pipefail
expected="${1:?expected version required}"
binary="${2:-target/release/aether-browser}"
set +e
actual="$($binary --version 2>&1)"
code=$?
set -e
printf 'AETHER_BROWSER_VERSION_SMOKE_EXPECTED=Aether Browser %s\n' "$expected"
printf 'AETHER_BROWSER_VERSION_SMOKE_ACTUAL=%s\n' "$actual"
printf 'AETHER_BROWSER_VERSION_SMOKE_EXIT=%s\n' "$code"
[[ $code -eq 0 && "$actual" == "Aether Browser ${expected}" ]]

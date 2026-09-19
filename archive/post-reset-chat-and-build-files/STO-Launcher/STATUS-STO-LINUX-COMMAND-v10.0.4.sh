#!/usr/bin/env bash
set -euo pipefail
prefix="${PREFIX:-$HOME/.local}"
state_file="${XDG_STATE_HOME:-$HOME/.local/state}/sto-linux-command/release-state"
printf 'STO_LINUX_COMMAND_EXPECTED=10.0.4\n'
printf 'AETHERFORGE_OS_EXPECTED=10.0.3\n'
if [[ -f "$state_file" ]]; then cat "$state_file"; else printf 'RELEASE_STATE=missing\n'; fi
[[ -x "$prefix/bin/sto-linux-gui" ]] && printf 'GUI=present\n' || printf 'GUI=missing\n'
[[ -x "$prefix/bin/sto-linux" ]] && printf 'CLI=present\n' || printf 'CLI=missing\n'
command -v umu-run >/dev/null 2>&1 && printf 'UMU=present\n' || printf 'UMU=missing\n'
command -v gamescope >/dev/null 2>&1 && printf 'GAMESCOPE=present\n' || printf 'GAMESCOPE=missing\n'
command -v xdotool >/dev/null 2>&1 && printf 'XDOTOOL=present\n' || printf 'XDOTOOL=missing\n'
[[ -n "${DISPLAY:-}" ]] && printf 'DISPLAY=%s\n' "$DISPLAY" || printf 'DISPLAY=missing\n'

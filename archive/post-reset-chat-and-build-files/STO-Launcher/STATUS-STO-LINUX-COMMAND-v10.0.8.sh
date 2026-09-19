#!/usr/bin/env bash
set -euo pipefail
prefix="${PREFIX:-$HOME/.local}"
state_file="${XDG_STATE_HOME:-$HOME/.local/state}/sto-linux-command/release-state"
data_root="${XDG_DATA_HOME:-$HOME/.local/share}/sto-linux"
log_root="${XDG_STATE_HOME:-$HOME/.local/state}/sto-linux/logs"
printf 'STO_LINUX_COMMAND_EXPECTED=10.0.8
'
printf 'AETHERFORGE_OS_EXPECTED=10.0.3
'
if [[ -f "$state_file" ]]; then cat "$state_file"; else printf 'RELEASE_STATE=missing
'; fi
[[ -x "$prefix/bin/sto-linux-gui" ]] && printf 'GUI=present
' || printf 'GUI=missing
'
[[ -x "$prefix/bin/sto-linux" ]] && printf 'CLI=present
' || printf 'CLI=missing
'
command -v umu-run >/dev/null 2>&1 && printf 'UMU=present
' || printf 'UMU=missing
'
command -v gamescope >/dev/null 2>&1 && printf 'GAMESCOPE_VISUAL_HOST=present
' || printf 'GAMESCOPE_VISUAL_HOST=missing
'
command -v xdotool >/dev/null 2>&1 && printf 'XDOTOOL_ARC_CONTAINMENT=present
' || printf 'XDOTOOL_ARC_CONTAINMENT=missing
'
[[ -n "${DISPLAY:-}" ]] && printf 'DISPLAY=%s
' "$DISPLAY" || printf 'DISPLAY=missing
'
printf 'SESSION_TYPE=%s\n' "${XDG_SESSION_TYPE:-unknown}"
printf 'X11_HOST_MARKER=%s\n' "${AETHERFORGE_STO_X11_HOST:-unset}"
printf 'MANAGED_PREFIX=%s
' "$data_root/prefixes/standalone"
printf 'STO_LAUNCH_LOG=%s
' "$log_root/sto-launch.log"
printf 'ARC_LAUNCH_LOG=%s
' "$log_root/arc-launch.log"
printf '%s
' '--- relevant processes ---'
pgrep -af 'Arc(Launcher|Client)?\.exe|ArcOSBrowser\.exe|Star Trek Online\.exe|GameClient\.exe|gamescope|umu-run' || true

lock_path="${XDG_CACHE_HOME:-$HOME/.cache}/sto-linux/installers/.arc-install.lock"
if [[ -f "$lock_path" ]]; then printf 'ARC_INSTALL_LOCK=present pid=%s\n' "$(head -n1 "$lock_path" 2>/dev/null || true)"; else printf 'ARC_INSTALL_LOCK=clear\n'; fi
printf 'CRYPTIC_FAST_LAUNCH_EXPECTED=AutoLaunch=1\n'
printf 'GAMESCOPE_KEEPALIVE_EXPECTED=enabled\n'

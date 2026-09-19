#!/usr/bin/env bash
set -euo pipefail
prefix="${PREFIX:-$HOME/.local}"
state_dir="${XDG_STATE_HOME:-$HOME/.local/state}/sto-linux-command"
state_file="$state_dir/release-state"
backup_root="$state_dir/backups"
fail(){ printf 'ERROR: %s
' "$*" >&2; exit 1; }
latest="$(find "$backup_root" -maxdepth 1 -type d -name 'pre-v10.0.5-*' -printf '%T@ %p\n' 2>/dev/null | sort -nr | head -n1 | cut -d' ' -f2-)"
[[ -n "$latest" && -d "$latest" ]] || fail "no pre-v10.0.5 backup found under $backup_root"
restored=0
for rel in bin/sto-linux bin/sto-linux-gui share/applications/sto-linux-command.desktop; do
  if [[ -e "$latest/$rel" ]]; then
    mkdir -p "$prefix/$(dirname "$rel")"
    cp -a "$latest/$rel" "$prefix/$rel"
    restored=1
  fi
done
[[ "$restored" -eq 1 ]] || fail "backup contains no restorable launcher files"
previous="10.0.4"
if [[ -f "$state_file" ]]; then
  candidate="$(awk -F= '$1=="MIGRATED_FROM" {print $2; exit}' "$state_file")"
  [[ -n "$candidate" && "$candidate" != "none" ]] && previous="$candidate"
fi
cat > "$state_file" <<EOF
STO_LINUX_COMMAND_VERSION=$previous
AETHERFORGE_OS_VERSION=10.0.3
ROLLBACK_FROM=10.0.5
ROLLBACK_SOURCE=$latest
ROLLBACK_RESULT=PASS
EOF
printf 'ROLLBACK_RESULT=PASS
'
printf 'ROLLBACK_SOURCE=%s
' "$latest"
printf 'RESTORED_VERSION=%s
' "$previous"

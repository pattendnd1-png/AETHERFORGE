#!/usr/bin/env bash
set -euo pipefail

PKG='aether-browser'
qkk_rc=${1:?usage: classify-pacman-qkk.sh <pacman-qkk-exit> <log-file>}
log=${2:?usage: classify-pacman-qkk.sh <pacman-qkk-exit> <log-file>}
[[ -f "$log" ]] || { echo 'AETHER_BROWSER_PACMAN_QKK=FAIL:missing-log'; exit 2; }

is_shared_dir() {
  case "$1" in
    /usr|/usr/bin|/usr/lib|/usr/lib/aetherforge|/usr/lib/systemd|/usr/lib/systemd/user|/usr/lib/udev|/usr/lib/udev/rules.d|/usr/share|/usr/share/applications|/usr/share/licenses)
      return 0 ;;
    *) return 1 ;;
  esac
}

allowed_paths=()
unexpected=()
summary=''
while IFS= read -r line || [[ -n "$line" ]]; do
  [[ -n "$line" ]] || continue
  if [[ "$line" =~ ^warning:\ aether-browser:\ (.*)\ \((UID\ mismatch|GID\ mismatch|Permissions\ mismatch|Modification\ time\ mismatch)\)$ ]]; then
    path=${BASH_REMATCH[1]}
    reason=${BASH_REMATCH[2]}
    if is_shared_dir "$path"; then
      printf 'AETHER_BROWSER_PACMAN_QKK_SHARED_DIR_METADATA=ALLOW:%s:%s\n' "$path" "$reason"
      seen=0
      for prior in "${allowed_paths[@]:-}"; do
        if [[ "$prior" == "$path" ]]; then seen=1; break; fi
      done
      (( seen == 1 )) || allowed_paths+=("$path")
    else
      unexpected+=("$line")
    fi
  elif [[ "$line" =~ ^aether-browser:\ [0-9]+\ total\ files,\ [0-9]+\ altered\ files?$ ]]; then
    summary=$line
  else
    unexpected+=("$line")
  fi
done < "$log"

if [[ -n "$summary" ]]; then
  printf 'AETHER_BROWSER_PACMAN_QKK_SUMMARY=%s\n' "$summary"
fi

if (( ${#unexpected[@]} > 0 )); then
  echo 'AETHER_BROWSER_PACMAN_QKK=FAIL:unexpected-integrity-output'
  echo 'AETHER_BROWSER_PACMAN_QKK_OUTPUT_BEGIN'
  printf '%s\n' "${unexpected[@]}"
  echo 'AETHER_BROWSER_PACMAN_QKK_OUTPUT_END'
  exit 1
fi

if (( qkk_rc == 0 )); then
  echo 'AETHER_BROWSER_PACMAN_QKK=PASS:clean'
  exit 0
fi

if (( ${#allowed_paths[@]} > 0 )); then
  printf 'AETHER_BROWSER_PACMAN_QKK=PASS_WITH_SHARED_DIR_METADATA:%d\n' "${#allowed_paths[@]}"
  exit 0
fi

echo 'AETHER_BROWSER_PACMAN_QKK=FAIL:nonzero-without-allowlisted-metadata'
exit 1

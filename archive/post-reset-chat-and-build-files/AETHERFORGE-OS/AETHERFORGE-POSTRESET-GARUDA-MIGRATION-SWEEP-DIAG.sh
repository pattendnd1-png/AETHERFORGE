#!/usr/bin/env bash
set -Eeuo pipefail
umask 077

REPO_FULL="${AETHERFORGE_REPO:-pattendnd1-png/AETHERFORGE}"
REPO_DIR="${AETHERFORGE_REPO_DIR:-$HOME/Downloads/AETHERFORGE}"
PROFILE_REPO="${AETHERFORGE_PROFILE_REPO:-pattendnd1-png/pattendnd1-png}"
PROFILE_DIR="${AETHERFORGE_PROFILE_DIR:-$HOME/.local/share/aetherforge/profile-readme}"
STATE_DIR="${XDG_STATE_HOME:-$HOME/.local/state}/aetherforge-github-sweep"
CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/aetherforge"
LOCK_FILE="$STATE_DIR/sweep.lock"
MAPPING_FILE="$CONFIG_DIR/postreset-source-map.tsv"
UPLOAD_STATE="$STATE_DIR/uploaded-artifacts.sha256"
RELEASE_TAG="${AETHERFORGE_WIP_RELEASE_TAG:-wip-post-reset}"
UPLOAD_ARTIFACTS="${AETHERFORGE_UPLOAD_ARTIFACTS:-1}"
MODE="${1:-full}"

DIAG_FILE="${AETHERFORGE_DIAG_FILE:-$HOME/Downloads/AETHERFORGE-MIGRATION-SWEEP-DIAG.txt}"
DIAG_TMP="${DIAG_FILE}.tmp"
HAD_AUTOSYNC=0
LAST_ERR_LINE=""
LAST_ERR_COMMAND=""
LAST_ERR_STATUS=""

mkdir -p "$(dirname -- "$DIAG_FILE")"
: > "$DIAG_TMP"
exec > >(tee -a "$DIAG_TMP") 2>&1

diag_header() {
  printf '%s\n' 'AETHERFORGE_MIGRATION_DIAGNOSTIC=START'
  printf 'started_at=%s\n' "$(date --iso-8601=seconds 2>/dev/null || date)"
  printf 'script=%s\n' "$0"
  printf 'mode=%s\n' "$MODE"
  printf 'user=%s\n' "$(id -un 2>/dev/null || true)"
  printf 'home=%s\n' "$HOME"
}

on_err() {
  local status=$? line="${BASH_LINENO[0]:-${LINENO:-unknown}}" cmd="${BASH_COMMAND:-unknown}"
  LAST_ERR_LINE="$line"
  LAST_ERR_COMMAND="$cmd"
  LAST_ERR_STATUS="$status"
  printf '\nAETHERFORGE_MIGRATION_ERROR=DETECTED\n' >&2
  printf 'error_status=%s\nerror_line=%s\nerror_command=%q\n' "$status" "$line" "$cmd" >&2
  return "$status"
}

restore_autosync_if_needed() {
  if [[ "${HAD_AUTOSYNC:-0}" == 1 ]]; then
    systemctl --user start aetherforge-git-autosync.timer >/dev/null 2>&1 || true
    HAD_AUTOSYNC=0
  fi
}

append_diagnostic_context() {
  local status="$1"
  set +e
  printf '\n===== AETHERFORGE MIGRATION DIAGNOSTIC CONTEXT =====\n'
  printf 'finished_at=%s\n' "$(date --iso-8601=seconds 2>/dev/null || date)"
  printf 'exit_status=%s\n' "$status"
  printf 'last_error_status=%s\n' "${LAST_ERR_STATUS:-none}"
  printf 'last_error_line=%s\n' "${LAST_ERR_LINE:-none}"
  printf 'last_error_command=%s\n' "${LAST_ERR_COMMAND:-none}"
  printf 'repo_full=%s\nrepo_dir=%s\nprofile_repo=%s\nreset_date=%s\n' \
    "${REPO_FULL:-unknown}" "${REPO_DIR:-unknown}" "${PROFILE_REPO:-unknown}" "${RESET_DATE:-unknown}"

  printf '\n--- OS ---\n'
  uname -a 2>&1 || true
  sed -n '1,40p' /etc/os-release 2>/dev/null || true
  [[ -f /etc/garuda-release ]] && cat /etc/garuda-release 2>/dev/null || true

  printf '\n--- Required commands ---\n'
  for c in git gh rsync python3 sha256sum flock systemctl journalctl; do
    if command -v "$c" >/dev/null 2>&1; then
      printf '%-12s %s\n' "$c" "$(command -v "$c")"
    else
      printf '%-12s MISSING\n' "$c"
    fi
  done
  git --version 2>&1 || true
  gh --version 2>&1 | head -n 2 || true
  rsync --version 2>&1 | head -n 2 || true
  python3 --version 2>&1 || true

  printf '\n--- GitHub authentication ---\n'
  gh auth status -h github.com 2>&1 || true

  printf '\n--- AETHERFORGE repository state ---\n'
  if [[ -d "${REPO_DIR:-}/.git" ]]; then
    git -C "$REPO_DIR" status --short --branch 2>&1 || true
    printf 'HEAD='; git -C "$REPO_DIR" rev-parse HEAD 2>&1 || true
    printf 'REMOTE_MAIN='; git -C "$REPO_DIR" ls-remote origin refs/heads/main 2>&1 | awk '{print $1}' || true
    printf '%s\n' '--- recent commits ---'
    git -C "$REPO_DIR" log -8 --oneline --decorate 2>&1 || true
    printf '%s\n' '--- remotes (credentials redacted) ---'
    git -C "$REPO_DIR" remote -v 2>&1 | sed -E 's#https://[^/@]+@github\.com/#https://github.com/#g' || true
    printf '%s\n' '--- working tree summary ---'
    git -C "$REPO_DIR" diff --stat 2>&1 || true
    git -C "$REPO_DIR" diff --cached --stat 2>&1 || true
  else
    echo 'repo_checkout=NOT_PRESENT'
  fi

  printf '\n--- Migration source map ---\n'
  if [[ -f "${MAPPING_FILE:-}" ]]; then
    sed -E 's#\t/home/[^/]+/#\t~/#' "$MAPPING_FILE" 2>&1 || true
  else
    echo 'mapping_file=NOT_PRESENT'
  fi

  printf '\n--- systemd user service state ---\n'
  systemctl --user status aetherforge-postreset-sweep.service --no-pager 2>&1 || true
  systemctl --user status aetherforge-postreset-sweep.timer --no-pager 2>&1 || true
  systemctl --user status aetherforge-git-autosync.service --no-pager 2>&1 || true
  systemctl --user status aetherforge-git-autosync.timer --no-pager 2>&1 || true

  printf '\n--- recent migration sweep journal ---\n'
  journalctl --user -u aetherforge-postreset-sweep.service -n 120 --no-pager 2>&1 || true
  printf '\n--- recent autosync journal ---\n'
  journalctl --user -u aetherforge-git-autosync.service -n 80 --no-pager 2>&1 || true

  printf '\n--- disk space ---\n'
  df -h "$HOME" 2>&1 || true

  printf '\nAETHERFORGE_MIGRATION_DIAGNOSTIC=END\n'
  set -e
}

on_exit() {
  local status=$?
  trap - ERR EXIT
  set +e
  restore_autosync_if_needed
  append_diagnostic_context "$status"
  cp -f "$DIAG_TMP" "$DIAG_FILE" 2>/dev/null || true
  chmod 600 "$DIAG_FILE" 2>/dev/null || true
  rm -f "$DIAG_TMP" 2>/dev/null || true
  printf '\nDIAGNOSTIC_FILE=%s\n' "$DIAG_FILE" >/dev/tty 2>/dev/null || true
  if [[ "$status" -eq 0 ]]; then
    printf 'MIGRATION_EXIT=PASS\n' >/dev/tty 2>/dev/null || true
  else
    printf 'MIGRATION_EXIT=FAIL:%s\n' "$status" >/dev/tty 2>/dev/null || true
  fi
  exit "$status"
}

trap on_err ERR
trap on_exit EXIT
diag_header

mkdir -p "$STATE_DIR" "$CONFIG_DIR" "$HOME/.local/bin" "$HOME/.config/systemd/user"
touch "$UPLOAD_STATE"

log() { printf '%s\n' "$*"; }
die() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
command_exists() { command -v "$1" >/dev/null 2>&1; }

root_birth_date() {
  local d
  d="$(stat -c %w / 2>/dev/null | cut -d' ' -f1 || true)"
  if [[ -n "$d" && "$d" != "-" && "$d" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}$ ]]; then
    printf '%s\n' "$d"
  else
    printf '%s\n' "2026-09-01"
  fi
}
RESET_DATE="${AETHERFORGE_RESET_DATE:-$(root_birth_date)}"

canonical_project_for_name() {
  local s="${1,,}"
  case "$s" in
    *aether*browser*|*aether-browser*) printf '%s' 'AetherBrowser' ;;
    *opendeck*) printf '%s' 'OpenDeck' ;;
    *forgehx*) printf '%s' 'ForgeHX' ;;
    *forgekonsole*|*aether*terminal*) printf '%s' 'ForgeKonsole' ;;
    *control*center*) printf '%s' 'Control-Center' ;;
    *aetherai*) printf '%s' 'AetherAI' ;;
    *aetherstream*) printf '%s' 'AetherStream' ;;
    *beacn*) printf '%s' 'BEACN-Control' ;;
    *behringer*) printf '%s' 'Behringer-Control' ;;
    *opensanctuary*) printf '%s' 'OpenSanctuary' ;;
    *sto*linux*command*|*sto*launcher*) printf '%s' 'STO-Launcher' ;;
    *reforge*logitech*) printf '%s' 'ReForge-Logitech' ;;
    *darkstone*) printf '%s' 'Darkstone-RS' ;;
    *stellar*online*) printf '%s' 'Stellar-Online' ;;
    *aetherforge*mobile*) printf '%s' 'AetherForge-Mobile' ;;
    *aetherdisplay*) printf '%s' 'AetherDisplay' ;;
    *aetherforge*) printf '%s' 'AETHERFORGE-OS' ;;
    *reforge*quake*|*quake*reforge*) printf '%s' 'ReForge-Quake' ;;
    *aetherforge*obs*|*obs*overlay*) printf '%s' 'OBS-Integration' ;;
    *) printf '%s' '' ;;
  esac
}

is_historical_core_artifact() {
  local b="$(basename -- "$1")"
  shopt -s nocasematch
  if [[ "$b" =~ ^AETHERFORGE[-_].*(V|v)?([0-7])([._-]|$) ]] ||
     [[ "$b" =~ (RC[0-9]|HOSTILE[-_ ]TAKEOVER|CLEAN[-_ ]REBASE|FULL[-_ ]TAKEOVER).*(V|v)?[0-7] ]] ||
     [[ "$b" =~ ^RUN-AETHERFORGE-V[3-7] ]] ||
     [[ "$b" =~ ^APPLY-AETHERFORGE-V[3-7] ]]; then
    shopt -u nocasematch
    return 0
  fi
  shopt -u nocasematch
  return 1
}

replace_managed_block() {
  local file="$1" body="$2"
  python3 - "$file" "$body" <<'PY'
import pathlib, sys
p=pathlib.Path(sys.argv[1]); body=sys.argv[2]
start='<!-- AETHERFORGE:AUTO:START -->'
end='<!-- AETHERFORGE:AUTO:END -->'
text=p.read_text() if p.exists() else ''
block=f"{start}\n{body.rstrip()}\n{end}"
if start in text and end in text:
    a=text.index(start); b=text.index(end,a)+len(end)
    text=text[:a]+block+text[b:]
else:
    if text and not text.endswith('\n'): text+='\n'
    text += ('\n' if text else '') + block + '\n'
p.write_text(text)
PY
}

safe_rsync_tree() {
  local src="$1" dst="$2"
  mkdir -p "$dst"
  rsync -a --delete-delay \
    --exclude='.git/' --exclude='.hg/' --exclude='.svn/' \
    --exclude='target/' --exclude='node_modules/' --exclude='dist/' --exclude='build/' \
    --exclude='out/' --exclude='.next/' --exclude='.cache/' --exclude='__pycache__/' \
    --exclude='.venv/' --exclude='venv/' --exclude='.tox/' --exclude='.pytest_cache/' \
    --exclude='.env' --exclude='.env.*' --exclude='*.pem' --exclude='*.key' --exclude='*.p12' \
    --exclude='id_rsa*' --exclude='id_ed25519*' --exclude='credentials*' --exclude='secrets*' \
    --exclude='*.iso' --exclude='*.img' --exclude='*.qcow2' --exclude='*.vdi' --exclude='*.vmdk' \
    --exclude='*.pkg.tar.zst' --exclude='*.AppImage' \
    "$src" "$dst"
}

is_source_root() {
  local d="$1"
  [[ -f "$d/Cargo.toml" || -f "$d/package.json" || -f "$d/pyproject.toml" || -f "$d/PKGBUILD" || -d "$d/crates" || -d "$d/src" ]]
}

candidate_epoch() {
  local d="$1" e
  if [[ -d "$d/.git" ]]; then
    e="$(git -C "$d" log -1 --format=%ct 2>/dev/null || true)"
    [[ "$e" =~ ^[0-9]+$ ]] && { printf '%s\n' "$e"; return; }
  fi
  stat -c %Y "$d" 2>/dev/null || printf '0\n'
}

discover_sources() {
  local tmp="$STATE_DIR/candidates.tsv"
  : > "$tmp"
  local roots=("$HOME/Downloads" "$HOME/Projects" "$HOME/src" "$HOME/dev" "$HOME/Documents")
  local root d proj epoch base
  for root in "${roots[@]}"; do
    [[ -d "$root" ]] || continue
    while IFS= read -r -d '' d; do
      [[ "$d" == "$REPO_DIR" || "$d" == "$REPO_DIR"/* ]] && continue
      is_source_root "$d" || continue
      base="$(basename -- "$d")"
      proj="$(canonical_project_for_name "$base")"
      [[ -n "$proj" ]] || continue
      epoch="$(candidate_epoch "$d")"
      printf '%s\t%s\t%s\n' "$proj" "$epoch" "$d" >> "$tmp"
    done < <(find "$root" -mindepth 1 -maxdepth 5 -type d \( \
      -iname '*aether*' -o -iname '*forge*' -o -iname '*opendeck*' -o -iname '*sanctuary*' -o \
      -iname '*darkstone*' -o -iname '*stellar*' -o -iname '*beacn*' -o -iname '*behringer*' -o -iname '*sto*' \
      \) -print0 2>/dev/null)
  done

  python3 - "$tmp" "$MAPPING_FILE" <<'PY'
import pathlib, sys
src=pathlib.Path(sys.argv[1]); out=pathlib.Path(sys.argv[2]); best={}
for line in src.read_text().splitlines():
    try: project, epoch, path=line.split('\t',2); epoch=int(epoch)
    except Exception: continue
    cur=best.get(project)
    if cur is None or epoch>cur[0]: best[project]=(epoch,path)
out.parent.mkdir(parents=True,exist_ok=True)
out.write_text(''.join(f'{p}\t{e}\t{path}\n' for p,(e,path) in sorted(best.items())))
PY
}

capture_garuda_base() {
  local d="$REPO_DIR/os/garuda-base"
  mkdir -p "$d/manifests" "$d/config"
  cp -f /etc/os-release "$d/config/os-release" 2>/dev/null || true
  [[ -f /etc/garuda-release ]] && cp -f /etc/garuda-release "$d/config/garuda-release" || true
  [[ -f /etc/pacman.conf ]] && cp -f /etc/pacman.conf "$d/config/pacman.conf" || true
  [[ -f /etc/mkinitcpio.conf ]] && cp -f /etc/mkinitcpio.conf "$d/config/mkinitcpio.conf" || true
  uname -a > "$d/manifests/kernel.txt"
  printf 'reset_baseline_date=%s\n' "$RESET_DATE" > "$d/manifests/post-reset-baseline.env"
  if command_exists pacman; then
    pacman -Qqe > "$d/manifests/pacman-explicit.txt" || true
    pacman -Qqm > "$d/manifests/pacman-foreign.txt" || true
  fi
  systemctl list-unit-files --state=enabled --no-pager 2>/dev/null > "$d/manifests/systemd-enabled.txt" || true
}

copy_textish_file() {
  local src="$1" dst="$2" mime
  [[ -f "$src" && -r "$src" ]] || return 0
  if command_exists file; then
    mime="$(file -b --mime-type "$src" 2>/dev/null || true)"
    case "$mime" in
      text/*|application/json|application/xml|application/x-shellscript|application/javascript|application/toml) ;;
      *) return 0 ;;
    esac
  fi
  mkdir -p "$(dirname -- "$dst")"
  cp -f -- "$src" "$dst"
}

capture_aetherforge_overlay() {
  local root dst f rel
  mkdir -p "$REPO_DIR/os/overlay" "$REPO_DIR/os/integration/system-files"
  local dirs=(/etc/aetherforge /usr/share/aetherforge /usr/local/share/aetherforge /opt/aetherforge "$HOME/.config/aetherforge")
  for root in "${dirs[@]}"; do
    [[ -d "$root" && -r "$root" ]] || continue
    dst="$REPO_DIR/os/overlay/$(printf '%s' "$root" | sed 's#^/##; s#/#__#g')"
    safe_rsync_tree "$root/" "$dst/" || true
    if command_exists file; then
      while IFS= read -r -d '' f; do
        case "$(file -b --mime-type "$f" 2>/dev/null || true)" in
          application/x-executable|application/x-pie-executable|application/x-sharedlib|application/octet-stream) rm -f -- "$f" ;;
        esac
      done < <(find "$dst" -type f -print0 2>/dev/null)
    fi
  done

  local scan_dirs=(/etc/systemd/system /usr/lib/systemd/system /etc/udev/rules.d /usr/lib/udev/rules.d /usr/share/applications "$HOME/.config/systemd/user")
  for root in "${scan_dirs[@]}"; do
    [[ -d "$root" ]] || continue
    while IFS= read -r -d '' f; do
      rel="$(printf '%s' "$f" | sed 's#^/##; s#/#__#g')"
      copy_textish_file "$f" "$REPO_DIR/os/integration/system-files/$rel"
    done < <(find "$root" -maxdepth 1 -type f \( -iname '*aether*' -o -iname '*forge*' -o -iname '*opendeck*' -o -iname '*beacn*' -o -iname '*behringer*' \) -print0 2>/dev/null)
  done
}

import_sources() {
  mkdir -p "$REPO_DIR/apps" "$REPO_DIR/migration"
  [[ -f "$MAPPING_FILE" ]] || return 0
  local proj epoch src dst head branch
  : > "$REPO_DIR/migration/POST_RESET_SOURCE_MAP.md"
  {
    echo '# Post-reset source map'
    echo
    echo '> Paths are intentionally omitted from this public manifest. The live Garuda host owns the private path mapping.'
    echo
    echo '| Project | Imported | Git HEAD |'
    echo '| --- | --- | --- |'
  } > "$REPO_DIR/migration/POST_RESET_SOURCE_MAP.md"
  while IFS=$'\t' read -r proj epoch src; do
    [[ -n "$proj" && -d "$src" ]] || continue
    if [[ "$proj" == 'AETHERFORGE-OS' ]]; then
      dst="$REPO_DIR/os/source"
    else
      dst="$REPO_DIR/apps/$proj"
    fi
    safe_rsync_tree "$src/" "$dst/"
    head="$(git -C "$src" rev-parse --short=12 HEAD 2>/dev/null || printf 'non-git')"
    branch="$(git -C "$src" branch --show-current 2>/dev/null || true)"
    printf '| %s | yes | `%s` |\n' "$proj" "$head" >> "$REPO_DIR/migration/POST_RESET_SOURCE_MAP.md"
    printf 'source_branch=%s\nsource_head=%s\n' "${branch:-unknown}" "$head" > "$dst/.aetherforge-migration-source"
  done < "$MAPPING_FILE"
}

import_postreset_loose_files() {
  local root="$HOME/Downloads" f b proj dest
  mkdir -p "$REPO_DIR/archive/post-reset-chat-and-build-files"
  [[ -d "$root" ]] || return 0
  while IFS= read -r -d '' f; do
    b="$(basename -- "$f")"
    is_historical_core_artifact "$b" && continue
    proj="$(canonical_project_for_name "$b")"
    if [[ -z "$proj" ]]; then
      [[ "${b,,}" == *aetherforge* ]] || continue
      proj='AETHERFORGE-OS'
    fi
    case "$b" in
      *.sh|*.rs|*.py|*.toml|*.json|*.md|*.txt|*.patch|*.diff|*.sha256|SHA256SUMS*|PKGBUILD)
        dest="$REPO_DIR/archive/post-reset-chat-and-build-files/$proj/$b"
        mkdir -p "$(dirname -- "$dest")"
        cp -f -- "$f" "$dest"
        ;;
    esac
  done < <(find "$root" -maxdepth 2 -type f -newermt "$RESET_DATE" -print0 2>/dev/null)
}

artifact_matches_project() {
  local b="$1"
  [[ -n "$(canonical_project_for_name "$b")" || "${b,,}" == *aetherforge* ]]
}

ensure_release() {
  [[ "$UPLOAD_ARTIFACTS" == 1 ]] || return 0
  gh release view "$RELEASE_TAG" -R "$REPO_FULL" >/dev/null 2>&1 && return 0
  gh release create "$RELEASE_TAG" -R "$REPO_FULL" --prerelease \
    --title 'AETHERFORGE WIP Rust Reforge — post-reset Garuda artifacts' \
    --notes 'Rolling work-in-progress artifacts for the post-reset AETHERFORGE Rust reforge. Garuda Linux/Arch is the OS-tree backbone. These assets are development outputs, not a finished distro release.' >/dev/null
}

upload_release_asset() {
  local f="$1" sha size tmp part note base
  [[ "$UPLOAD_ARTIFACTS" == 1 ]] || return 0
  sha="$(sha256sum "$f" | awk '{print $1}')"
  grep -Fxq "$sha" "$UPLOAD_STATE" && return 0
  size="$(stat -c %s "$f")"
  base="$(basename -- "$f")"
  ensure_release
  if (( size < 1900000000 )); then
    gh release upload "$RELEASE_TAG" "$f" -R "$REPO_FULL" --clobber
  else
    tmp="$(mktemp -d)"
    split -b 1800M -d -a 3 --additional-suffix='.part' "$f" "$tmp/$base."
    note="$tmp/$base.REASSEMBLE.txt"
    {
      echo "Original: $base"
      echo "SHA256: $sha"
      echo "Reassemble: cat '$base.'*.part > '$base'"
      echo "Then verify with: sha256sum '$base'"
    } > "$note"
    for part in "$tmp"/*; do gh release upload "$RELEASE_TAG" "$part" -R "$REPO_FULL" --clobber; done
    rm -rf "$tmp"
  fi
  printf '%s\n' "$sha" >> "$UPLOAD_STATE"
}

sweep_artifacts() {
  local root="$HOME/Downloads" f b sha size proj
  mkdir -p "$REPO_DIR/artifacts"
  local index="$REPO_DIR/artifacts/INDEX.tsv"
  printf 'project\tfilename\tbytes\tsha256\tstorage\n' > "$index"
  [[ -d "$root" ]] || return 0
  while IFS= read -r -d '' f; do
    b="$(basename -- "$f")"
    is_historical_core_artifact "$b" && continue
    artifact_matches_project "$b" || continue
    case "$b" in
      *.iso|*.img|*.pkg.tar.zst|*.zip|*.tar.gz|*.tar.xz|*.tar.zst|*.AppImage)
        sha="$(sha256sum "$f" | awk '{print $1}')"
        size="$(stat -c %s "$f")"
        proj="$(canonical_project_for_name "$b")"; [[ -n "$proj" ]] || proj='AETHERFORGE-OS'
        printf '%s\t%s\t%s\t%s\tGitHub prerelease %s\n' "$proj" "$b" "$size" "$sha" "$RELEASE_TAG" >> "$index"
        upload_release_asset "$f"
        ;;
    esac
  done < <(find "$root" -maxdepth 2 -type f -newermt "$RESET_DATE" -print0 2>/dev/null)
}

append_managed_gitignore() {
  local f="$REPO_DIR/.gitignore"
  touch "$f"
  python3 - "$f" <<'PY'
import pathlib,sys
p=pathlib.Path(sys.argv[1]); t=p.read_text()
s='\n# BEGIN AETHERFORGE POST-RESET MANAGED IGNORE\n'
e='# END AETHERFORGE POST-RESET MANAGED IGNORE\n'
body='''# Build/cache output\n**/target/\n**/node_modules/\n**/dist/\n**/build/\n**/.cache/\n**/__pycache__/\n**/.venv/\n# Secrets/private material\n**/.env\n**/.env.*\n**/*.pem\n**/*.key\n**/*.p12\n**/id_rsa*\n**/id_ed25519*\n# Large generated artifacts live in GitHub Releases\n*.iso\n*.img\n*.qcow2\n*.vdi\n*.vmdk\n*.pkg.tar.zst\n*.AppImage\n'''
block=s+body+e
if '# BEGIN AETHERFORGE POST-RESET MANAGED IGNORE' in t:
    a=t.index('# BEGIN AETHERFORGE POST-RESET MANAGED IGNORE')
    b=t.index('# END AETHERFORGE POST-RESET MANAGED IGNORE',a)+len('# END AETHERFORGE POST-RESET MANAGED IGNORE')
    t=t[:a]+block.strip('\n')+t[b:]
else:
    if t and not t.endswith('\n'): t+='\n'
    t+=block
p.write_text(t)
PY
}

strong_secret_scan() {
  local out="$STATE_DIR/secret-scan.txt"
  : > "$out"
  local pat='(gh[pousr]_[A-Za-z0-9]{10,}|github_pat_[A-Za-z0-9_]{20,}|sk-[A-Za-z0-9_-]{12,}|AKIA[0-9A-Z]{16}|-----BEGIN (RSA |OPENSSH |EC )?PRIVATE KEY-----|OPENAI_API_KEY[[:space:]]*=[[:space:]]*[^${][^[:space:]]+)'
  grep -RIE --binary-files=without-match --exclude-dir=.git "$pat" "$REPO_DIR" > "$out" 2>/dev/null || true
  if [[ -s "$out" ]]; then
    cat "$out" >&2
    die 'Potential secret detected. Nothing was committed or pushed.'
  fi
}

project_version() {
  local d="$1" v
  if [[ -f "$d/Cargo.toml" ]]; then
    v="$(awk -F'=' '/^[[:space:]]*version[[:space:]]*=/{gsub(/["[:space:]]/,"",$2); print $2; exit}' "$d/Cargo.toml")"
    [[ -n "$v" ]] && { printf '%s' "$v"; return; }
  fi
  if [[ -f "$d/package.json" ]]; then
    python3 - "$d/package.json" <<'PY' 2>/dev/null || true
import json,sys
try: print(json.load(open(sys.argv[1])).get('version',''))
except Exception: pass
PY
    return
  fi
  printf '%s' 'WIP'
}

generate_repo_readme() {
  local readme="$REPO_DIR/README.md" body tmp app version rust_count os_files app_files artifact_count
  [[ -f "$readme" ]] || printf '# AETHERFORGE\n\nCustom Linux OS and first-party software ecosystem.\n' > "$readme"
  tmp="$(mktemp)"
  os_files="$(find "$REPO_DIR/os" -type f 2>/dev/null | wc -l)"
  app_files="$(find "$REPO_DIR/apps" -type f 2>/dev/null | wc -l)"
  artifact_count="$(tail -n +2 "$REPO_DIR/artifacts/INDEX.tsv" 2>/dev/null | wc -l || true)"
  {
    echo '> [!IMPORTANT]'
    echo '> **AETHERFORGE is a work-in-progress RUST reforge.** The post-reset OS tree uses **Garuda Linux / Arch Linux as its backbone** while AETHERFORGE replaces/reforges the user-facing OS layer, services, tooling, integration, and first-party applications. This repository tracks active development; it is not a claim of a finished or fully qualified distro release.'
    echo
    echo '## Live development status'
    echo
    echo "- **OS backbone:** Garuda Linux / Arch Linux"
    echo "- **Development model:** Work-in-progress RUST reforge"
    echo "- **Post-reset baseline:** \`$RESET_DATE\`"
    echo "- **OS/source files tracked:** $os_files"
    echo "- **Application source files tracked:** $app_files"
    echo "- **Current release/build artifacts indexed:** $artifact_count"
    echo "- **Large build outputs:** GitHub prerelease \`$RELEASE_TAG\`"
    echo
    echo '## Active source tree'
    echo
    echo '| Component | Version/status | Rust files |'
    echo '| --- | --- | ---: |'
    if [[ -d "$REPO_DIR/apps" ]]; then
      while IFS= read -r -d '' app; do
        version="$(project_version "$app")"; [[ -n "$version" ]] || version='WIP'
        rust_count="$(find "$app" -type f -name '*.rs' 2>/dev/null | wc -l)"
        printf '| `%s` | %s | %s |\n' "$(basename "$app")" "$version" "$rust_count"
      done < <(find "$REPO_DIR/apps" -mindepth 1 -maxdepth 1 -type d -print0 | sort -z)
    fi
    echo
    echo '## Repository layout'
    echo
    echo '- `os/` — Garuda-backed AETHERFORGE OS overlay, system configuration, services, integration and reproducibility manifests.'
    echo '- `apps/` — current post-reset first-party application source trees.'
    echo '- `artifacts/` — checksums/index for evolving builds; large binaries are attached to the WIP GitHub prerelease.'
    echo '- `archive/post-reset-chat-and-build-files/` — current-era scripts, patches, verification files and handoff artifacts recovered from local development output.'
    echo '- `migration/` — public migration/source mapping without private machine paths.'
    echo
    echo '## Migration policy'
    echo
    echo 'Current post-reset source wins. Pre-reset RC3/v4/v5/v6/v7 hostile-takeover and clean-rebase lineages are not imported into the active tree. Transient caches and private credentials are never committed.'
  } > "$tmp"
  body="$(cat "$tmp")"; rm -f "$tmp"
  replace_managed_block "$readme" "$body"
}

ensure_profile_repo() {
  gh repo view "$PROFILE_REPO" >/dev/null 2>&1 || gh repo create "$PROFILE_REPO" --public --description 'GitHub profile and live AETHERFORGE development progress' >/dev/null
  if [[ -d "$PROFILE_DIR/.git" ]]; then
    git -C "$PROFILE_DIR" pull --rebase --autostash >/dev/null 2>&1 || true
  else
    rm -rf "$PROFILE_DIR"
    gh repo clone "$PROFILE_REPO" "$PROFILE_DIR" >/dev/null
  fi
}

update_profile_readme() {
  ensure_profile_repo
  local f="$PROFILE_DIR/README.md" tmp body latest apps osfiles
  [[ -f "$f" ]] || printf '# pattendnd1-png\n' > "$f"
  latest="$(git -C "$REPO_DIR" rev-parse --short=12 HEAD 2>/dev/null || printf 'pending')"
  apps="$(find "$REPO_DIR/apps" -mindepth 1 -maxdepth 1 -type d 2>/dev/null | wc -l)"
  osfiles="$(find "$REPO_DIR/os" -type f 2>/dev/null | wc -l)"
  tmp="$(mktemp)"
  {
    echo '## AETHERFORGE — live development'
    echo
    echo '**Work-in-progress RUST reforge** using **Garuda Linux / Arch Linux as the OS-tree backbone**.'
    echo
    echo "- Active first-party app trees: **$apps**"
    echo "- AETHERFORGE OS/config/integration files tracked: **$osfiles**"
    echo "- Current AETHERFORGE commit: \`$latest\`"
    echo "- Development artifacts: rolling \`$RELEASE_TAG\` prerelease"
    echo
    echo "Main repository: https://github.com/$REPO_FULL"
    echo
    echo '> Development is ongoing. Status shown here reflects the post-reset Garuda-backed AETHERFORGE tree and does not imply final release qualification.'
  } > "$tmp"
  body="$(cat "$tmp")"; rm -f "$tmp"
  replace_managed_block "$f" "$body"
  if ! git -C "$PROFILE_DIR" diff --quiet -- README.md || [[ -n "$(git -C "$PROFILE_DIR" status --porcelain README.md)" ]]; then
    git -C "$PROFILE_DIR" add README.md
    git -C "$PROFILE_DIR" -c user.name='AETHERFORGE Automation' -c user.email='actions@users.noreply.github.com' commit -m 'Update AETHERFORGE development progress' >/dev/null
    git -C "$PROFILE_DIR" push origin HEAD:main >/dev/null
  fi
}

install_validation_workflow() {
  mkdir -p "$REPO_DIR/.github/workflows"
  cat > "$REPO_DIR/.github/workflows/main.yml" <<'YML'
name: AETHERFORGE Repository Validation
on:
  push:
    branches: [main]
  pull_request:
    branches: [main]
  workflow_dispatch:
permissions:
  contents: read
jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7
      - name: Confirm WIP Rust/Garuda identity
        shell: bash
        run: |
          set -euo pipefail
          grep -qi 'work-in-progress RUST reforge' README.md
          grep -qi 'Garuda Linux / Arch Linux' README.md
          test -f os/garuda-base/manifests/post-reset-baseline.env
      - name: Reject paid OpenAI workflow dependencies
        shell: bash
        run: |
          set -euo pipefail
          paid_secret='OPENAI''_API_KEY'; paid_host='api''.'openai'.com'; paid_route='/v1/'responses
          pattern="${paid_secret}|${paid_host//./\\.}|${paid_route}"
          if grep -RIE "$pattern" .github/workflows 2>/dev/null; then exit 1; fi
      - name: Reject oversized Git objects
        shell: bash
        run: |
          set -euo pipefail
          bad=0
          while IFS= read -r -d '' f; do
            size=$(stat -c %s "$f")
            if (( size > 95000000 )); then echo "Oversized Git file: $f ($size bytes)"; bad=1; fi
          done < <(find . -type f -not -path './.git/*' -print0)
          exit "$bad"
      - name: Validate active shell source
        shell: bash
        run: |
          set -euo pipefail
          count=0
          for root in os apps; do
            [[ -d "$root" ]] || continue
            while IFS= read -r -d '' f; do bash -n "$f"; count=$((count+1)); done < <(find "$root" -type f -name '*.sh' -print0)
          done
          echo "SHELL_FILES_VALIDATED=$count"
      - name: Repository gate
        shell: bash
        run: |
          set -euo pipefail
          test -f LICENSE
          test -f README.md
          test -d os
          test -d apps
          test -f artifacts/INDEX.tsv
          echo 'AETHERFORGE_POSTRESET_GARUDA_REPOSITORY=PASS'
YML
}

commit_and_push_repo() {
  strong_secret_scan
  git -C "$REPO_DIR" add -A
  if git -C "$REPO_DIR" diff --cached --quiet; then
    log 'AETHERFORGE_MIGRATION_CHANGES=NONE'
    return 0
  fi
  git -C "$REPO_DIR" -c user.name='AETHERFORGE Automation' -c user.email='actions@users.noreply.github.com' commit -m 'Migrate post-reset Garuda-backed AETHERFORGE Rust reforge' >/dev/null
  git -C "$REPO_DIR" pull --rebase --autostash origin main >/dev/null
  git -C "$REPO_DIR" push origin HEAD:main >/dev/null
}

install_recurring_sweep() {
  local installed="$HOME/.local/bin/aetherforge-postreset-sweep"
  cp -f "$0" "$installed"
  chmod +x "$installed"
  cat > "$HOME/.config/systemd/user/aetherforge-postreset-sweep.service" <<EOFUNIT
[Unit]
Description=AETHERFORGE post-reset Garuda source/artifact sweep
After=network-online.target

[Service]
Type=oneshot
ExecStart=$installed sync
EOFUNIT
  cat > "$HOME/.config/systemd/user/aetherforge-postreset-sweep.timer" <<'EOFUNIT'
[Unit]
Description=Periodically synchronize AETHERFORGE post-reset source and progress

[Timer]
OnBootSec=2min
OnUnitActiveSec=2min
Persistent=true
Unit=aetherforge-postreset-sweep.service

[Install]
WantedBy=timers.target
EOFUNIT
  systemctl --user daemon-reload
  systemctl --user enable --now aetherforge-postreset-sweep.timer >/dev/null
}

ensure_repo() {
  gh auth status -h github.com >/dev/null 2>&1 || die 'GitHub CLI is not authenticated.'
  if [[ -d "$REPO_DIR/.git" ]]; then
    git -C "$REPO_DIR" pull --rebase --autostash origin main >/dev/null 2>&1 || true
  else
    rm -rf "$REPO_DIR"
    mkdir -p "$(dirname -- "$REPO_DIR")"
    gh repo clone "$REPO_FULL" "$REPO_DIR" >/dev/null
  fi
}

run_sweep() {
  command_exists git || die 'git is required.'
  command_exists gh || die 'gh is required.'
  command_exists rsync || die 'rsync is required (sudo pacman -S --needed rsync).'
  command_exists python3 || die 'python3 is required.'
  command_exists sha256sum || die 'sha256sum is required.'
  command_exists flock || die 'flock is required.'

  exec 9>"$LOCK_FILE"
  flock -n 9 || { log 'AETHERFORGE_SWEEP=SKIP_ALREADY_RUNNING'; exit 0; }

  log 'AETHERFORGE_POSTRESET_GARUDA_MIGRATION=START'
  log "RESET_BASELINE=$RESET_DATE"
  ensure_repo

  # Avoid commit races with the previously installed 30-second autosync while this sweep stages a coherent migration.
  HAD_AUTOSYNC=0
  if systemctl --user is-enabled aetherforge-git-autosync.timer >/dev/null 2>&1; then
    HAD_AUTOSYNC=1
    systemctl --user stop aetherforge-git-autosync.timer >/dev/null 2>&1 || true
    systemctl --user stop aetherforge-git-autosync.service >/dev/null 2>&1 || true
  fi
  append_managed_gitignore
  discover_sources
  capture_garuda_base
  capture_aetherforge_overlay
  import_sources
  import_postreset_loose_files
  sweep_artifacts
  generate_repo_readme
  install_validation_workflow
  commit_and_push_repo
  update_profile_readme

  if [[ "$MODE" == full ]]; then install_recurring_sweep; fi

  if [[ $HAD_AUTOSYNC == 1 ]]; then
    systemctl --user start aetherforge-git-autosync.timer >/dev/null 2>&1 || true
    HAD_AUTOSYNC=0
  fi

  local local_sha remote_sha
  local_sha="$(git -C "$REPO_DIR" rev-parse HEAD)"
  remote_sha="$(git -C "$REPO_DIR" ls-remote origin refs/heads/main | awk '{print $1}')"
  [[ "$local_sha" == "$remote_sha" ]] || die 'Remote main does not match local migration commit.'

  log 'GARUDA_OS_TREE_BACKBONE=PASS'
  log 'AETHERFORGE_WIP_RUST_REFORGE=PASS'
  log 'POSTRESET_SOURCE_MIGRATION=PASS'
  log 'PROFILE_PROGRESS_SYNC=PASS'
  log "REMOTE_MAIN=$remote_sha"
  log 'AETHERFORGE_GITHUB_MIGRATION_SWEEP=PASS'
}

if [[ "${AETHERFORGE_LIB_ONLY:-0}" == 1 ]]; then
  return 0 2>/dev/null || exit 0
fi

run_sweep

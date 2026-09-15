#!/usr/bin/env bash
set -Eeuo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"
PACKAGES_DIR="${AETHERFORGE_PACKAGES_DIR:-$ROOT/out/packages}"
mkdir -p "$PACKAGES_DIR" "$ROOT/qualification"

usage(){
  cat <<'EOF'
Usage: ./BUILD-AETHERFORGE.sh <check|preflight|packages|repo|iso-from-repo|iso|all-from-repo|all|vm-diagnostics-from-repo|vm-diagnostics>

  check     standard-library unittest + source/TOML/shell qualification
  preflight source + managed-project + supplemental hardware evidence + installer/ISO readiness
  packages  check, run native build gates, and build every stable Arch package
  repo      packages, build pacman repository databases, qualify package graph
  iso-from-repo reuse an already-GREEN repo stage, then build/inspect the exact ISO
  iso       repo, then build/inspect the ArchISO image with mkarchiso
  all-from-repo same mandatory gates as iso-from-repo; VM is not required
  all       full mandatory source/build/package/repo/exact-ISO qualification; VM is not required
  vm-diagnostics-from-repo  OPTIONAL: iso-from-repo plus QEMU smoke/install diagnostics
  vm-diagnostics            OPTIONAL: full build plus QEMU smoke/install diagnostics
EOF
}

REMOVED_CHAT_TOKEN="forge""cord"

assert_removed_legacy_chat_stack(){
  local order stale
  order="$(python3 packaging/build-package.py --order)"
  if grep -qi -- "$REMOVED_CHAT_TOKEN" <<<"$order"; then
    echo "ERROR: removed legacy chat packages reappeared in the package build order." >&2
    echo "$order" >&2
    exit 1
  fi
  stale="$(find packages integrations sources -maxdepth 2 -iname "*${REMOVED_CHAT_TOKEN}*" -print -quit 2>/dev/null || true)"
  if [[ -n "$stale" ]]; then
    echo "ERROR: stale removed-chat path remains in the active stable tree: $stale" >&2
    echo "Reapply the total-excision hotfix before building." >&2
    exit 1
  fi
}

check_stage(){
  echo '[Aetherforge] Source qualification (standard-library unittest + legacy function contracts)'
  PYTHONDONTWRITEBYTECODE=1 python3 tools/qualify-stable-source.py
  python3 tools/write_release_status.py
}

preflight_stage(){
  check_stage
  echo '[Aetherforge] Supplemental read-only hardware evidence'
  if ! python3 tools/qualify-hardware.py; then
    echo '[Aetherforge] Supplemental hardware evidence did not pass; continuing mandatory release qualification.' >&2
  fi
  echo '[Aetherforge] Installer/ISO source + environment preflight'
  python3 tools/qualify-installer-iso-preflight.py
  python3 tools/write_release_status.py
}

packages_stage(){
  assert_removed_legacy_chat_stack
  preflight_stage
  ./tools/qualify-build.sh
  rm -f "$PACKAGES_DIR"/*.pkg.tar.*
  mapfile -t packages < <(python3 packaging/build-package.py --order)
  for pkg in "${packages[@]}"; do
    echo "[Aetherforge] Building Arch package: $pkg"
    rm -f "packages/$pkg"/*.pkg.tar.* 2>/dev/null || true
    python3 packaging/build-package.py "$pkg"
    built=()
    while IFS= read -r -d '' artifact; do
      [[ "$artifact" == *.sig ]] && continue
      if bsdtar -xOf "$artifact" .PKGINFO 2>/dev/null | grep -Fxq "pkgname = $pkg"; then
        built+=("$artifact")
      fi
    done < <(find "packages/$pkg" -maxdepth 1 -type f -name '*.pkg.tar.*' -print0)
    if ((${#built[@]} != 1)); then
      echo "ERROR: expected one primary built artifact for $pkg, found ${#built[@]}" >&2
      echo "Discovered package artifacts:" >&2
      find "packages/$pkg" -maxdepth 1 -type f -name '*.pkg.tar.*' -printf '  %f\n' >&2 || true
      exit 1
    fi
    cp -f "${built[0]}" "$PACKAGES_DIR/"
  done
}

repo_stage(){
  packages_stage
  python3 packaging/build-repositories.py --packages-dir "$PACKAGES_DIR" --repo-root "$ROOT/repo"
  python3 packaging/build-offline-repository.py --repo-root "$ROOT/repo" --output "$ROOT/repo/aetherforge-offline"
  ./tools/qualify-packages.sh
  python3 tools/write_release_status.py
}

iso_from_repo_stage(){
  assert_removed_legacy_chat_stack
  check_stage
  echo '[Aetherforge] Reusing previously-qualified 7.0.7-stable repositories'
  python3 tools/qualify-existing-repo.py
  ./tools/qualify-iso.sh
  python3 tools/write_release_status.py
}

iso_stage(){
  repo_stage
  ./tools/qualify-iso.sh
  python3 tools/write_release_status.py
}

run_vm_gates(){
  command -v qemu-system-x86_64 >/dev/null 2>&1 || { echo 'OPTIONAL VM DIAGNOSTIC UNAVAILABLE: qemu-system-x86_64 is not installed' >&2; exit 1; }
  iso_path="$(python3 - <<'PY2'
import json
print(json.load(open('qualification/iso.json'))['iso'])
PY2
)"
  ./tools/qemu-smoke-test.sh --iso "$iso_path"
  ./tools/qualify-install-vm.sh --iso "$iso_path"
  python3 tools/write_release_status.py
}

all_from_repo_stage(){
  iso_from_repo_stage
}

all_stage(){
  iso_stage
}

vm_diagnostics_from_repo_stage(){
  iso_from_repo_stage
  run_vm_gates
}

vm_diagnostics_stage(){
  iso_stage
  run_vm_gates
}

case "${1:-}" in
  check) check_stage ;;
  preflight) preflight_stage ;;
  packages) packages_stage ;;
  repo) repo_stage ;;
  iso-from-repo) iso_from_repo_stage ;;
  iso) iso_stage ;;
  all-from-repo) all_from_repo_stage ;;
  all) all_stage ;;
  vm-diagnostics-from-repo) vm_diagnostics_from_repo_stage ;;
  vm-diagnostics) vm_diagnostics_stage ;;
  -h|--help|help|'') usage; [[ -n "${1:-}" ]] || exit 2 ;;
  *) echo "Unknown command: $1" >&2; usage >&2; exit 2 ;;
esac

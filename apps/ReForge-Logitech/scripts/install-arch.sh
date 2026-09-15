#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="0.6.1"
ARCH_DIR="${ROOT}/packaging/arch"
TARBALL="${ROOT}/dist/reforge-logitech-linux-${VERSION}.tar.gz"
LOCAL_TARBALL="${ARCH_DIR}/reforge-logitech-linux-${VERSION}.tar.gz"

if [[ ! -e /etc/arch-release ]]; then
  printf '%s\n' 'ERROR: this installer is intended for Arch Linux and Arch-based distributions.' >&2
  exit 1
fi
if [[ ${EUID} -eq 0 ]]; then
  printf '%s\n' 'ERROR: run this script as your normal desktop user, not with sudo.' >&2
  exit 1
fi
if ! command -v makepkg >/dev/null 2>&1; then
  printf '%s\n' 'ERROR: makepkg is missing. Install base-devel first: sudo pacman -S --needed base-devel' >&2
  exit 1
fi

"${ROOT}/scripts/make-dist.sh" >/dev/null
cp -f "${TARBALL}" "${LOCAL_TARBALL}"
trap 'rm -f "${LOCAL_TARBALL}"' EXIT

(
  cd "${ARCH_DIR}"
  makepkg -si --clean --cleanbuild --syncdeps --needed
)

sudo udevadm control --reload-rules
sudo udevadm trigger --subsystem-match=hidraw
systemctl --user daemon-reload
systemctl --user enable reforge-logitechd.service
systemctl --user restart reforge-logitechd.service

printf '\nInstalled ReForge Logitech Linux %s.\n' "${VERSION}"
printf '%s\n' 'Check it with: reforge-logitechctl health'
printf '%s\n' 'List devices:  reforge-logitechctl devices'
printf '%s\n' 'Launch GUI:    reforge-logitech'
printf '%s\n' 'Lighting CLI: reforge-logitechctl lighting get 1'

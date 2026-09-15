#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="0.6.1"
NAME="reforge-logitech-linux-${VERSION}"
OUT="${ROOT}/dist/${NAME}.tar.gz"

mkdir -p "${ROOT}/dist"
rm -f "${OUT}"

tar \
  --exclude='./.git' \
  --exclude='./.worktrees' \
  --exclude='./target' \
  --exclude='./dist' \
  --exclude='./packaging/arch/*.tar.gz' \
  --exclude='./packaging/arch/*.pkg.tar.zst' \
  --transform="s,^\.,${NAME}," \
  -czf "${OUT}" \
  -C "${ROOT}" .

printf '%s\n' "${OUT}"

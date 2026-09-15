#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
command -v makepkg >/dev/null || { echo 'makepkg is required (Arch: pacman -S base-devel).' >&2; exit 1; }

ARCHIVE="$($ROOT/scripts/create-source-package.sh)"
cp "$ARCHIVE" "$ROOT/forgehx-10.0.31.tar.gz"
trap 'rm -f "$ROOT/forgehx-10.0.31.tar.gz"' EXIT
makepkg -sf
mkdir -p "$ROOT/dist"
find "$ROOT" -maxdepth 1 -type f -name 'forgehx-*.pkg.tar.*' -exec cp -f {} "$ROOT/dist/" \;
echo 'Arch package copied to dist/.'

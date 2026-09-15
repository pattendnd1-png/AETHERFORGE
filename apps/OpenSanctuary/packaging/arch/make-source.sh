#!/usr/bin/env bash
set -euo pipefail
script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd -- "${script_dir}/../.." && pwd)"
version="0.4.2"
name="OpenSanctuary-${version}"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
mkdir -p "${tmp}/${name}"
tar -C "$root" --exclude=.git --exclude=.worktrees --exclude='packaging/arch/*.tar.gz' -cf - . | tar -C "${tmp}/${name}" -xf -
tar -C "$tmp" -czf "${script_dir}/${name}.tar.gz" "$name"
echo "Created ${script_dir}/${name}.tar.gz"
sha256sum "${script_dir}/${name}.tar.gz"
echo "Run: cd ${script_dir} && makepkg -si"

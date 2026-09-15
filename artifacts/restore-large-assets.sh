#!/usr/bin/env bash
set -euo pipefail
REPO="${AETHERFORGE_REPO:-pattendnd1-png/AETHERFORGE}"
ROOT="${1:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
MANIFEST="$ROOT/artifacts/LARGE-FILES.tsv"
[[ -f "$MANIFEST" ]] || { echo "Missing $MANIFEST" >&2; exit 1; }
command -v gh >/dev/null || { echo 'gh is required.' >&2; exit 1; }
tail -n +2 "$MANIFEST" | while IFS=$'\t' read -r rel bytes sha tag asset; do
  [[ -n "$rel" && -n "$asset" ]] || continue
  dst="$ROOT/$rel"; mkdir -p "$(dirname -- "$dst")"
  tmp="$(mktemp)"
  if gh release download "$tag" -R "$REPO" -p "$asset" -O "$tmp" >/dev/null 2>&1; then
    got="$(sha256sum "$tmp" | awk '{print $1}')"
    [[ "$got" == "$sha" ]] || { rm -f "$tmp"; echo "Checksum mismatch: $rel" >&2; exit 1; }
    mv -f "$tmp" "$dst"
    echo "RESTORED=$rel"
  else
    rm -f "$tmp"
    echo "MISSING_RELEASE_ASSET=$asset" >&2
    exit 1
  fi
done

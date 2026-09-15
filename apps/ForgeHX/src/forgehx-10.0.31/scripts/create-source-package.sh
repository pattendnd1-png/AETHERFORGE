#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
VERSION="10.0.31"
NAME="forgehx-$VERSION"
DIST="$ROOT/dist"
STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT
mkdir -p "$DIST" "$STAGE/$NAME"

tar -C "$ROOT" \
  --exclude='.git' \
  --exclude='.worktrees' \
  --exclude='target' \
  --exclude='dist' \
  -cf - . | tar -C "$STAGE/$NAME" -xf -

tar -C "$STAGE" -czf "$DIST/$NAME.tar.gz" "$NAME"
echo "$DIST/$NAME.tar.gz"

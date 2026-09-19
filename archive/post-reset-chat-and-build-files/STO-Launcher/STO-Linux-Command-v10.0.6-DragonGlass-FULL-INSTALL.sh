#!/usr/bin/env bash
set -euo pipefail
VERSION="10.0.6"
ARCHIVE="sto-linux-command-10.0.6.tar.gz"
EXPECTED_SHA256="f66fcf24bc9f98a870ef743e399f66ca0e838a5f0839f36dfe4e3bd449b49114"
MODE="${1:---install}"
SELF_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORK_ROOT="${XDG_CACHE_HOME:-$HOME/.cache}/sto-linux-command/v$VERSION"
SOURCE_DIR="$WORK_ROOT/sto-linux-command-10.0.6"
say(){ printf '==> %s\n' "$*"; }
fail(){ printf 'ERROR: %s\n' "$*" >&2; exit 1; }
verify_archive(){
  [[ -f "$SELF_DIR/$ARCHIVE" ]] || fail "missing $SELF_DIR/$ARCHIVE"
  actual="$(sha256sum "$SELF_DIR/$ARCHIVE" | awk '{print $1}')"
  [[ "$actual" == "$EXPECTED_SHA256" ]] || fail "source archive SHA-256 mismatch"
  say "source archive SHA-256 OK"
}
extract_source(){
  rm -rf "$WORK_ROOT"
  mkdir -p "$WORK_ROOT"
  tar -xzf "$SELF_DIR/$ARCHIVE" -C "$WORK_ROOT"
  [[ -f "$SOURCE_DIR/scripts/install-v10006-dragon-glass.sh" ]] || fail "10.0.6 host installer missing after extraction"
}
case "$MODE" in
  --self-test) verify_archive; extract_source; (cd "$SOURCE_DIR" && bash scripts/install-v10006-dragon-glass.sh --self-test) ;;
  --build-test) verify_archive; extract_source; (cd "$SOURCE_DIR" && bash scripts/install-v10006-dragon-glass.sh --build-test) ;;
  --install|--all) verify_archive; extract_source; (cd "$SOURCE_DIR" && bash scripts/install-v10006-dragon-glass.sh --all) ;;
  *) fail "usage: $0 [--self-test|--build-test|--install|--all]" ;;
esac

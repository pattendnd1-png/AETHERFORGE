#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
VERSION=0.2.2
PLATFORM=linux-x86_64
OUT_DIR=${AETHERAI_OUTPUT_DIR:-$HOME/Downloads}
ARTIFACT="AetherAI-v$VERSION-$PLATFORM.tar.gz"
ARCHIVE="$OUT_DIR/$ARTIFACT"
VERIFY_OUT="$OUT_DIR/AetherAI-v$VERSION-$PLATFORM-VERIFY.txt"
HOST_LOG_OUT="$OUT_DIR/AetherAI-v$VERSION-$PLATFORM-HOST-VERIFY.txt"
SHA_OUT="$OUT_DIR/AetherAI-v$VERSION-$PLATFORM-SHA256SUMS.txt"

if [[ ${1:-} == --plan ]]; then
  cat <<PLAN
AETHERAI_VERSION=$VERSION
AETHERAI_PLATFORM=$PLATFORM
AETHERAI_ARTIFACT=$ARTIFACT
AETHERAI_BUNDLED_RUST=YES
AETHERAI_VENDOR_DEPENDENCIES=YES
AETHERAI_LIVE_ASSET_PINNING=YES
AETHERAI_INSTALL_LIVE_ASSETS=${AETHERAI_INSTALL_LIVE_ASSETS:-0}
AETHERAI_ARCHIVE_ON_VERIFY_FAILURE=YES
PLAN
  exit 0
fi
need(){ command -v "$1" >/dev/null 2>&1 || { echo "AETHERAI_LINUX_BUILDER_MISSING_TOOL=$1" >&2; exit 127; }; }
for tool in tar sha256sum tee find; do need "$tool"; done
mkdir -p "$OUT_DIR"
cd "$ROOT"
if [[ ${AETHERAI_INSTALL_LIVE_ASSETS:-0} == 1 ]]; then
  "$ROOT/scripts/acquire-live-assets.sh"
fi
rm -f "$ROOT/AetherAI-v$VERSION-VERIFY.txt"
set +e
"$ROOT/scripts/verify-linux-self-contained.sh" 2>&1 | tee "$HOST_LOG_OUT"
VERIFY_RC=${PIPESTATUS[0]}
set -e
REPORT_ROOT="$ROOT/AetherAI-v$VERSION-VERIFY.txt"
if [[ ! -f "$REPORT_ROOT" ]]; then
  { echo "AETHERAI_VERSION=$VERSION"; echo AETHERAI_V0_2_2_BUILD_VERIFY=FAIL; echo AETHERAI_V0_2_2_VERIFY=FAIL; } > "$REPORT_ROOT"
fi
cp -f "$REPORT_ROOT" "$VERIFY_OUT"
{
  echo "AETHERAI_PLATFORM_ARTIFACT=$ARTIFACT"
  if (( VERIFY_RC == 0 )); then echo AETHERAI_LINUX_HOST_VERIFY=PASS; else echo "AETHERAI_LINUX_HOST_VERIFY=FAIL:$VERIFY_RC"; fi
  echo AETHERAI_LINUX_RELEASE_ASSEMBLY=PASS
} >> "$VERIFY_OUT"
rm -rf "$ROOT/target/debug" "$ROOT/target/tmp" "$ROOT/target/.fingerprint" 2>/dev/null || true
rm -f "$ARCHIVE" "$SHA_OUT"
BASE=$(basename "$ROOT")
CANONICAL_ROOT="AetherAI-v$VERSION"
(
  cd "$(dirname "$ROOT")"
  tar \
    --exclude="$BASE/.aetherai" \
    --exclude="$BASE/target/debug" \
    --exclude="$BASE/target/tmp" \
    --exclude="$BASE/target/.fingerprint" \
    --transform="s,^$BASE,$CANONICAL_ROOT," \
    -czf "$ARCHIVE" "$BASE"
)
(cd "$OUT_DIR" && sha256sum "$ARTIFACT" "$(basename "$VERIFY_OUT")" "$(basename "$HOST_LOG_OUT")" > "$(basename "$SHA_OUT")")
echo "AETHERAI_LINUX_RELEASE=$ARCHIVE"
echo "AETHERAI_LINUX_VERIFY=$VERIFY_OUT"
echo "AETHERAI_HOST_VERIFY_LOG=$HOST_LOG_OUT"
echo "AETHERAI_LINUX_SHA256=$SHA_OUT"
echo AETHERAI_LINUX_RELEASE_ASSEMBLY=PASS
exit "$VERIFY_RC"

#!/usr/bin/env bash
set -euo pipefail
fail(){ echo "AETHER_BROWSER_V2_1_60_RELEASE_PACKAGE_OBSERVABILITY=FAIL:$1"; exit 1; }
ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
VERSION=$(sed -n "s/^VERSION='\([^']*\)'.*/\1/p" "$ROOT/scripts/package-release.sh" | head -1)
[[ -n "$VERSION" ]] || fail version
PREFIX="Aether-Browser-v${VERSION}"
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
OUT="$TMP/out"; BIN="$TMP/bin"; mkdir -p "$OUT" "$BIN"
printf 'ORIGINAL-SOURCE-ZIP\n' > "$OUT/${PREFIX}-source.zip"
original_sha=$(sha256sum "$OUT/${PREFIX}-source.zip" | awk '{print $1}')
cat > "$BIN/cargo" <<'SH'
#!/usr/bin/env bash
echo 'AETHER_BROWSER_FAKE_FMT_FAILURE=42'
exit 42
SH
chmod +x "$BIN/cargo"
set +e
PATH="$BIN:$PATH" bash "$ROOT/scripts/package-release.sh" "$OUT" > "$TMP/terminal.log" 2>&1
rc=$?
set -e
[[ "$rc" -eq 42 ]] || fail "exit:$rc"
grep -qF 'AETHER_BROWSER_FAKE_FMT_FAILURE=42' "$TMP/terminal.log" || fail hidden-failure-output
grep -qF 'AETHER_BROWSER_RELEASE_STATIC_VERIFY=FAIL:42' "$TMP/terminal.log" || fail missing-failure-marker
[[ -f "$OUT/${PREFIX}-STATIC-VERIFY.txt" ]] || fail missing-static-diagnostic
grep -qF 'AETHER_BROWSER_FAKE_FMT_FAILURE=42' "$OUT/${PREFIX}-STATIC-VERIFY.txt" || fail static-missing-diagnostic
new_sha=$(sha256sum "$OUT/${PREFIX}-source.zip" | awk '{print $1}')
[[ "$new_sha" == "$original_sha" ]] || fail source-zip-mutated
printf 'AETHER_BROWSER_V2_1_60_RELEASE_PACKAGE_OBSERVABILITY=PASS\n'

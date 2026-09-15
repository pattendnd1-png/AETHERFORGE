#!/usr/bin/env bash
set -euo pipefail
SRC=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TMP/pkg/scripts" "$TMP/pkg/tools/rust/x86_64-unknown-linux-gnu/bin"
cp "$SRC/scripts/aetherai-rust" "$TMP/pkg/scripts/aetherai-rust"
chmod +x "$TMP/pkg/scripts/aetherai-rust"
cat > "$TMP/pkg/tools/rust/x86_64-unknown-linux-gnu/bin/cargo" <<'FAKE'
#!/usr/bin/env bash
printf 'SYSROOT=%s\n' "${SYSROOT:-}"
FAKE
chmod +x "$TMP/pkg/tools/rust/x86_64-unknown-linux-gnu/bin/cargo"
out=$(PATH=/usr/bin:/bin "$TMP/pkg/scripts/aetherai-rust" check)
expected="$TMP/pkg/tools/rust/x86_64-unknown-linux-gnu"
grep -Fx "SYSROOT=$expected" <<<"$out"
echo AETHERAI_LINUX_CLIPPY_SYSROOT_ENV_TEST=PASS

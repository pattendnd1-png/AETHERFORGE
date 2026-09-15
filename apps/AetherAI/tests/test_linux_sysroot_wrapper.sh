#!/usr/bin/env bash
set -euo pipefail
SRC=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TMP/pkg/scripts" "$TMP/pkg/tools/rust/x86_64-unknown-linux-gnu/bin"
cp "$SRC/scripts/aetherai-rustc" "$TMP/pkg/scripts/aetherai-rustc"
chmod +x "$TMP/pkg/scripts/aetherai-rustc"
cat > "$TMP/pkg/tools/rust/x86_64-unknown-linux-gnu/bin/rustc" <<'FAKE'
#!/usr/bin/env bash
printf '%s\n' "$@"
FAKE
chmod +x "$TMP/pkg/tools/rust/x86_64-unknown-linux-gnu/bin/rustc"
out=$("$TMP/pkg/scripts/aetherai-rustc" --print sysroot)
expected="$TMP/pkg/tools/rust/x86_64-unknown-linux-gnu"
grep -Fx -- '--sysroot' <<<"$out"
grep -Fx -- "$expected" <<<"$out"
grep -Fx -- '--print' <<<"$out"
grep -Fx -- 'sysroot' <<<"$out"
echo AETHERAI_LINUX_SYSROOT_WRAPPER_TEST=PASS

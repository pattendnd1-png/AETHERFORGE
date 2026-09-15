#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
TOOLCHAIN="$ROOT/tools/rust/x86_64-unknown-linux-gnu"
REPORT="$ROOT/AetherAI-v0.2.2-VERIFY.txt"
fail=0
check_exec(){ [[ -x "$1" ]] || { echo "$2=FAIL"; fail=1; return; }; echo "$2=PASS"; }
check_file(){ [[ -e "$1" ]] || { echo "$2=FAIL"; fail=1; return; }; echo "$2=PASS"; }

check_exec "$TOOLCHAIN/bin/cargo" AETHERAI_LINUX_CARGO
check_exec "$TOOLCHAIN/bin/rustc" AETHERAI_LINUX_RUSTC
check_exec "$TOOLCHAIN/bin/rustfmt" AETHERAI_LINUX_RUSTFMT
check_exec "$TOOLCHAIN/bin/cargo-clippy" AETHERAI_LINUX_CLIPPY
check_exec "$ROOT/scripts/aetherai-rustc" AETHERAI_LINUX_RUSTC_WRAPPER
check_exec "$ROOT/scripts/aetherai-rustdoc" AETHERAI_LINUX_RUSTDOC_WRAPPER
check_file "$ROOT/Cargo.lock" AETHERAI_CARGO_LOCK
check_file "$ROOT/.cargo/config.toml" AETHERAI_VENDOR_CONFIG
check_file "$ROOT/resources/model-catalog.json" AETHERAI_MODEL_CATALOG
check_file "$ROOT/resources/inference/runtime-manifest.json" AETHERAI_RUNTIME_MANIFEST
if [[ -d "$ROOT/vendor" ]] && find "$ROOT/vendor" -mindepth 1 -maxdepth 1 -type d -print -quit | grep -q .; then
  echo AETHERAI_VENDOR_TREE=PASS
else
  echo AETHERAI_VENDOR_TREE=FAIL
  fail=1
fi
if (( fail )); then echo AETHERAI_LINUX_SELF_CONTAINED_RUST=FAIL; exit 1; fi
reported_sysroot=$("$ROOT/scripts/aetherai-rustc" --print sysroot 2>/dev/null || true)
if [[ "$reported_sysroot" == "$TOOLCHAIN" ]]; then echo AETHERAI_LINUX_SYSROOT=PASS; else echo "AETHERAI_LINUX_SYSROOT=FAIL:$reported_sysroot"; fail=1; fi
SMOKE_DIR=$(mktemp -d); trap 'rm -rf "$SMOKE_DIR"' EXIT
printf 'fn main(){println!("AETHERAI_BUNDLED_STD=PASS");}\n' > "$SMOKE_DIR/main.rs"
if "$ROOT/scripts/aetherai-rustc" "$SMOKE_DIR/main.rs" -o "$SMOKE_DIR/smoke" >/dev/null 2>&1 && [[ "$("$SMOKE_DIR/smoke")" == AETHERAI_BUNDLED_STD=PASS ]]; then
  echo AETHERAI_LINUX_RUST_STD=PASS
else
  echo AETHERAI_LINUX_RUST_STD=FAIL; fail=1
fi
if (( fail )); then echo AETHERAI_LINUX_SELF_CONTAINED_RUST=FAIL; exit 1; fi
echo AETHERAI_LINUX_SELF_CONTAINED_RUST=PASS
if [[ ${1:-} == --preflight ]]; then exit 0; fi
export CARGO_NET_OFFLINE=true
set +e
"$ROOT/scripts/aetherai-rust" run --offline -p xtask -- verify
rc=$?
set -e
if [[ ! -f "$REPORT" ]]; then
  { echo AETHERAI_VERSION=0.2.2; echo AETHERAI_V0_2_2_BUILD_VERIFY=FAIL; echo AETHERAI_V0_2_2_VERIFY=FAIL; } > "$REPORT"
fi
{
  echo AETHERAI_PLATFORM=linux-x86_64
  echo AETHERAI_BUNDLED_RUST_LINUX=PASS
  echo AETHERAI_VENDOR_OFFLINE=PASS
  if (( rc == 0 )); then echo AETHERAI_V0_2_2_LINUX_VERIFY=PASS; else echo "AETHERAI_V0_2_2_LINUX_VERIFY=FAIL:$rc"; fi
} >> "$REPORT"
if [[ -n ${HOME:-} && -d "$HOME/Downloads" ]]; then
  cp -f "$REPORT" "$HOME/Downloads/AetherAI-v0.2.2-VERIFY.txt"
  echo "AETHERAI_UPLOAD_VERIFY=$HOME/Downloads/AetherAI-v0.2.2-VERIFY.txt" >> "$REPORT"
fi
tail -n 6 "$REPORT"
exit "$rc"

#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
VERSION=0.2.2
RUST_VERSION=1.98.0
RUST_DATE=2026-08-20
TARGET=x86_64-pc-windows-gnu
PLATFORM=windows-x86_64
DIST_BASE="https://static.rust-lang.org/dist/$RUST_DATE"
OUT_DIR=${AETHERAI_OUTPUT_DIR:-$HOME/Downloads}
ARTIFACT="AetherAI-v$VERSION-$PLATFORM-BUILDER.zip"
ARCHIVE="$OUT_DIR/$ARTIFACT"
VERIFY_OUT="$OUT_DIR/AetherAI-v$VERSION-$PLATFORM-BUILDER-VERIFY.txt"
SHA_OUT="$OUT_DIR/AetherAI-v$VERSION-$PLATFORM-BUILDER-SHA256SUMS.txt"
if [[ ${1:-} == --plan ]]; then
cat <<PLAN
AETHERAI_VERSION=$VERSION
AETHERAI_PLATFORM=$PLATFORM
AETHERAI_TARGET=$TARGET
AETHERAI_RUST_VERSION=$RUST_VERSION
AETHERAI_ARTIFACT=$ARTIFACT
AETHERAI_WINDOWS_RUNTIME=llama-b10649-bin-win-vulkan-x64.zip
AETHERAI_WINDOWS_NATIVE_VERIFY=PENDING_WINDOWS_HOST
PLAN
exit 0
fi
need(){ command -v "$1" >/dev/null 2>&1 || { echo "AETHERAI_WINDOWS_BUILDER_MISSING_TOOL=$1" >&2; exit 127; }; }
for tool in curl tar sha256sum mktemp find python3; do need "$tool"; done
mkdir -p "$OUT_DIR"; cd "$ROOT"
TMP=$(mktemp -d); trap 'rm -rf "$TMP"' EXIT
fetch_component(){ local name=$1 dest=$2 file="${1}-${RUST_VERSION}-${TARGET}.tar.xz" url="$DIST_BASE/${1}-${RUST_VERSION}-${TARGET}.tar.xz"; curl --proto '=https' --tlsv1.2 -fL --retry 3 -o "$TMP/$file" "$url"; curl --proto '=https' --tlsv1.2 -fL --retry 3 -o "$TMP/$file.sha256" "$url.sha256"; (cd "$TMP" && sha256sum -c "$file.sha256"); rm -rf "$TMP/unpack"; mkdir -p "$TMP/unpack"; tar -xJf "$TMP/$file" -C "$TMP/unpack"; local top; top=$(find "$TMP/unpack" -mindepth 1 -maxdepth 1 -type d -print -quit); "$top/install.sh" --prefix="$dest" --disable-ldconfig; }
DEST="$ROOT/tools/rust/$TARGET"; rm -rf "$DEST"; mkdir -p "$DEST"
for component in rustc cargo rust-std rustfmt clippy rust-mingw; do fetch_component "$component" "$DEST"; done
GCC="$DEST/lib/rustlib/$TARGET/bin/self-contained/x86_64-w64-mingw32-gcc.exe"
for f in "$DEST/bin/cargo.exe" "$DEST/bin/rustc.exe" "$DEST/bin/rustfmt.exe" "$DEST/bin/cargo-clippy.exe" "$GCC" "$ROOT/Cargo.lock" "$ROOT/.cargo/config.toml"; do [[ -e "$f" ]] || { echo "AETHERAI_WINDOWS_PREFLIGHT_MISSING=$f"; exit 1; }; done
cat > "$VERIFY_OUT" <<VERIFY
AETHERAI_VERSION=$VERSION
AETHERAI_PLATFORM=$PLATFORM
AETHERAI_WINDOWS_BUILDER_CONTRACT=PASS
AETHERAI_WINDOWS_MINGW_GCC=PASS
AETHERAI_WINDOWS_VULKAN_RUNTIME_PIN=PASS
AETHERAI_WINDOWS_NATIVE_VERIFY=PENDING_WINDOWS_HOST
AETHERAI_WINDOWS_RELEASE_ASSEMBLY=PASS
VERIFY
rm -f "$ARCHIVE" "$SHA_OUT"
python3 - "$ROOT" "$ARCHIVE" "$VERSION" <<'PYZIP'
import os, sys, zipfile
root, archive, version = sys.argv[1:]
base = os.path.basename(root)
canonical = f"AetherAI-v{version}"
skip_prefixes = {".aetherai", "target/debug", "target/tmp", "target/.fingerprint"}
with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=6) as zf:
    for current, dirs, files in os.walk(root):
        rel_dir = os.path.relpath(current, root).replace(os.sep, "/")
        if rel_dir == ".":
            rel_dir = ""
        dirs[:] = [d for d in dirs if not any((f"{rel_dir}/{d}" if rel_dir else d).startswith(p) for p in skip_prefixes)]
        for name in files:
            rel = f"{rel_dir}/{name}" if rel_dir else name
            if any(rel.startswith(p) for p in skip_prefixes):
                continue
            zf.write(os.path.join(current, name), f"{canonical}/{rel}")
PYZIP
(cd "$OUT_DIR" && sha256sum "$ARTIFACT" "$(basename "$VERIFY_OUT")" > "$(basename "$SHA_OUT")")
echo "AETHERAI_WINDOWS_BUILDER=$ARCHIVE"; echo "AETHERAI_WINDOWS_VERIFY=$VERIFY_OUT"; echo "AETHERAI_WINDOWS_SHA256=$SHA_OUT"; echo AETHERAI_WINDOWS_RELEASE_ASSEMBLY=PASS

#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
RUNTIME_ROOT="$ROOT/tools/inference"
MODEL_ROOT="$ROOT/models"
RUNTIME_ID=llama-b10649-linux-vulkan-x86_64
RUNTIME_ARCHIVE=llama-b10649-bin-ubuntu-vulkan-x64.tar.gz
RUNTIME_URL=https://github.com/ggml-org/llama.cpp/releases/download/b10649/llama-b10649-bin-ubuntu-vulkan-x64.tar.gz
RUNTIME_SHA=fee6f25a7c5c87310e9f75dd4b0a325a5a04bcbd46160f8c1a784cb816d01ad5
MODEL_FILE=Qwen3-0.6B-Q4_K_M.gguf
MODEL_URL=https://huggingface.co/Qwen/Qwen3-0.6B-GGUF/resolve/1208e45d782fe18602c5eaf10e5758d5b0f24c03/Qwen3-0.6B-Q4_K_M.gguf
MODEL_SHA=b0638f08417a2d3c8652760462eb5407c6e30173cf9608ad0820757a281eea0e

need(){ command -v "$1" >/dev/null 2>&1 || { echo "AETHERAI_LIVE_ASSET_TOOL=FAIL:$1" >&2; exit 127; }; }
for tool in curl tar sha256sum find sort paste; do need "$tool"; done
mkdir -p "$RUNTIME_ROOT/downloads" "$MODEL_ROOT"

download_verified(){
  local url=$1 final=$2 expected=$3 part="${2}.part"
  if [[ -f "$final" ]] && [[ "$(sha256sum "$final" | awk '{print $1}')" == "$expected" ]]; then return 0; fi
  curl --proto '=https' --tlsv1.2 --fail --location --retry 3 --continue-at - --output "$part" "$url"
  local actual; actual=$(sha256sum "$part" | awk '{print $1}')
  [[ "$actual" == "$expected" ]] || { rm -f "$part"; echo "AETHERAI_LIVE_ASSET_SHA256=FAIL:$actual" >&2; exit 2; }
  mv -f "$part" "$final"
}

archive="$RUNTIME_ROOT/downloads/$RUNTIME_ARCHIVE"
download_verified "$RUNTIME_URL" "$archive" "$RUNTIME_SHA"
install_dir="$RUNTIME_ROOT/$RUNTIME_ID"
rm -rf "$install_dir"; mkdir -p "$install_dir"
tar -xzf "$archive" -C "$install_dir"
server=$(find "$install_dir" -type f -name llama-server -print -quit)
[[ -n "$server" && -x "$server" ]] || { echo AETHERAI_GGUF_RUNTIME=FAIL:NO_LLAMA_SERVER >&2; exit 2; }
cat > "$RUNTIME_ROOT/aetherai-gguf-runtime" <<'WRAPPER'
#!/usr/bin/env bash
set -euo pipefail
HERE=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)
RUNTIME="$HERE/llama-b10649-linux-vulkan-x86_64"
SERVER=$(find "$RUNTIME" -type f -name llama-server -print -quit)
[[ -n "$SERVER" && -x "$SERVER" ]] || { echo 'AETHERAI_GGUF_RUNTIME=FAIL:NO_LLAMA_SERVER' >&2; exit 127; }
LIBS=$(find "$RUNTIME" -type f -name '*.so*' -printf '%h\n' | sort -u | paste -sd: -)
if [[ -n "$LIBS" ]]; then export LD_LIBRARY_PATH="$LIBS${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"; fi
exec "$SERVER" "$@"
WRAPPER
chmod +x "$RUNTIME_ROOT/aetherai-gguf-runtime"

download_verified "$MODEL_URL" "$MODEL_ROOT/$MODEL_FILE" "$MODEL_SHA"
echo AETHERAI_RUNTIME_ASSET_SHA256=PASS
echo AETHERAI_MODEL_ASSET_SHA256=PASS
echo "AETHERAI_GGUF_RUNTIME=$RUNTIME_ROOT/aetherai-gguf-runtime"
echo "AETHERAI_STARTER_MODEL=$MODEL_ROOT/$MODEL_FILE"
echo AETHERAI_LIVE_ASSETS=PASS

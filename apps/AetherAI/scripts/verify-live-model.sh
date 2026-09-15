#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
BIN="$ROOT/target/release/aetherai"
RUNTIME=${AETHERAI_GGUF_RUNTIME:-$ROOT/tools/inference/aetherai-gguf-runtime}
MODEL=${AETHERAI_TEST_GGUF:-$ROOT/models/Qwen3-0.6B-Q4_K_M.gguf}
DATA=${AETHERAI_LIVE_VERIFY_DATA_DIR:-$ROOT/.aetherai/live-verify}

if [[ ${AETHERAI_INSTALL_LIVE_ASSETS:-0} == 1 ]]; then
  "$ROOT/scripts/acquire-live-assets.sh"
fi
if [[ ! -x "$BIN" || ! -x "$RUNTIME" || ! -f "$MODEL" ]]; then
  echo AETHERAI_MODEL_SERVICE=FAIL:NO_TEST_MODEL
  echo AETHERAI_LIVE_MODEL_LOAD=FAIL:NO_TEST_MODEL
  echo AETHERAI_LIVE_GENERATION=FAIL:NO_TEST_MODEL
  echo AETHERAI_TOKEN_STREAM=FAIL:NO_TEST_MODEL
  exit 3
fi
rm -rf "$DATA"; mkdir -p "$DATA"
import_out=$("$BIN" --data-dir "$DATA" --import-model "$MODEL" --diagnostic 2>&1) || { echo "$import_out"; exit 1; }
model_id=$(printf '%s\n' "$import_out" | sed -n 's/^AETHERAI_MODEL_IMPORTED=//p' | tail -n1)
if [[ -z "$model_id" ]]; then
  echo AETHERAI_MODEL_SERVICE=FAIL:IMPORT
  echo AETHERAI_LIVE_MODEL_LOAD=FAIL:IMPORT
  echo AETHERAI_LIVE_GENERATION=FAIL:IMPORT
  echo AETHERAI_TOKEN_STREAM=FAIL:IMPORT
  exit 1
fi
set +e
verify_out=$(AETHERAI_GGUF_RUNTIME="$RUNTIME" "$BIN" --data-dir "$DATA" --verify-provider --model "$model_id" 2>&1)
rc=$?
set -e
printf '%s\n' "$verify_out"
if (( rc != 0 )); then exit "$rc"; fi
for endpoint in AETHERAI_MODEL_SERVICE AETHERAI_LIVE_MODEL_LOAD AETHERAI_LIVE_GENERATION AETHERAI_TOKEN_STREAM; do
  grep -qx "$endpoint=PASS" <<<"$verify_out" || { echo "$endpoint=FAIL:MISSING_ENDPOINT"; exit 1; }
done

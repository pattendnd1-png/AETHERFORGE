#!/usr/bin/env bash
set -euo pipefail

CRATE='servo-media-gstreamer'
CRATE_VERSION='0.5.0'
MODE="${AETHER_BROWSER_MEDIA_RENDERER:-cpu-bgra}"
CARGO_HOME_DIR="${CARGO_HOME:-$HOME/.cargo}"
REGISTRY_ROOT="$CARGO_HOME_DIR/registry/src"
VENDOR_DIR="vendor/${CRATE}-${CRATE_VERSION}"

if [[ "$MODE" != 'cpu-bgra' && "$MODE" != 'gpu-dmabuf' ]]; then
  echo "AETHER_BROWSER_MEDIA_SAFE_PREPARE=FAIL:invalid-renderer:$MODE"
  exit 2
fi

src=''
if [[ -d "$REGISTRY_ROOT" ]]; then
  src=$(find "$REGISTRY_ROOT" -mindepth 2 -maxdepth 2 -type d -name "${CRATE}-${CRATE_VERSION}" -print -quit 2>/dev/null || true)
fi
if [[ -z "$src" || ! -d "$src" ]]; then
  echo "AETHER_BROWSER_MEDIA_SAFE_PREPARE=FAIL:registry-source-not-found"
  echo "AETHER_BROWSER_MEDIA_SAFE_EXPECTED=${CRATE}-${CRATE_VERSION}"
  exit 3
fi

rm -rf "$VENDOR_DIR"
mkdir -p vendor
cp -a "$src" "$VENDOR_DIR"

python3 - "$VENDOR_DIR/render.rs" <<'PY'
from pathlib import Path
import re, sys
path = Path(sys.argv[1])
if not path.is_file():
    raise SystemExit(f'AETHER_BROWSER_MEDIA_SAFE_PREPARE=FAIL:missing:{path}')
text = path.read_text()
pattern = re.compile(
    r'(pub fn create_render\(gl_context: Box<dyn PlayerGLContext>\) -> Option<Render> \{\s*)'
    r'Render::new\(gl_context\)(\s*\})',
    re.MULTILINE,
)
replacement = (
    r'\1if std::env::var("AETHER_BROWSER_MEDIA_RENDERER").ok().as_deref() == Some("cpu-bgra") {\n'
    r'                None\n'
    r'            } else {\n'
    r'                Render::new(gl_context)\n'
    r'            }\2'
)
patched, count = pattern.subn(replacement, text, count=1)
if count != 1:
    raise SystemExit('AETHER_BROWSER_MEDIA_SAFE_PREPARE=FAIL:linux-create-render-pattern-not-found')
path.write_text(patched)
PY

python3 - <<'PY'
from pathlib import Path
manifest = Path('Cargo.toml')
text = manifest.read_text()
entry = 'servo-media-gstreamer = { path = "vendor/servo-media-gstreamer-0.5.0" }'
if '[patch.crates-io]' not in text:
    text = text.rstrip() + '\n\n[patch.crates-io]\n' + entry + '\n'
elif entry not in text:
    text = text.rstrip() + '\n' + entry + '\n'
manifest.write_text(text)
PY

rm -f Cargo.lock
printf 'AETHER_BROWSER_MEDIA_SAFE_PREPARE=PASS\n'
printf 'AETHER_BROWSER_MEDIA_RENDERER=%s\n' "$MODE"
printf 'AETHER_BROWSER_MEDIA_SAFE_VENDOR=%s\n' "$VENDOR_DIR"
printf 'AETHER_BROWSER_MEDIA_SAFE_PATCH=SERVO_GSTREAMER_CPU_BGRA_OPT_OUT\n'

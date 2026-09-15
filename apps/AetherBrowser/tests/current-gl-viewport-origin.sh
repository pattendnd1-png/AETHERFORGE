#!/usr/bin/env bash
set -euo pipefail
fail(){ echo "AETHER_BROWSER_GL_VIEWPORT_ORIGIN=FAIL:$1"; exit 1; }
LIVE=crates/aether-engine-servo/src/live.rs
# Browser geometry is top-left; OpenGL framebuffer geometry is bottom-left.
grep -qF 'fn content_blit_rect_for_size' "$LIVE" || fail no-gl-blit-transform
grep -qF 'size.height.saturating_sub(top.saturating_add(height))' "$LIVE" || fail no-top-to-bottom-left-conversion
grep -qF 'render_to_parent(gl.as_ref(), self.content_blit_rect());' "$LIVE" || fail compositor-still-using-logical-rect
grep -qF 'fn content_capture_rect' "$LIVE" || fail no-capture-transform
grep -qF 'self.content_blit_rect()' "$LIVE" || fail capture-not-using-gl-geometry
grep -qF 'AETHER_BROWSER_VIEWPORT_ORIGIN=PASS:' "$LIVE" || fail no-logical-origin-runtime-marker
grep -qF 'AETHER_BROWSER_VIEWPORT_BOTTOM=PASS:' "$LIVE" || fail no-bottom-runtime-marker
echo 'AETHER_BROWSER_GL_VIEWPORT_ORIGIN=PASS'

#!/usr/bin/env bash
set -euo pipefail
fail(){ echo "AETHER_BROWSER_NATIVE_CHROME_RESET=FAIL:$1"; exit 1; }
UI=crates/aether-ui/src/lib.rs
ENGINE=crates/aether-engine-servo/src/live.rs
UICARGO=crates/aether-ui/Cargo.toml
ENGCARGO=crates/aether-engine-servo/Cargo.toml
[[ ! -e crates/aether-ui/assets/browser.html ]] || fail browser-html-still-present
! grep -qF 'render_document' "$UI" || fail html-render-document-still-present
! grep -qF 'render_patch_script' "$UI" || fail dom-patch-still-present
! grep -qF 'ChromeDelegate' "$ENGINE" || fail chrome-webview-delegate-still-present
! grep -qF 'chrome_webview' "$ENGINE" || fail chrome-webview-still-present
! grep -qF 'chrome_context' "$ENGINE" || fail chrome-offscreen-context-still-present
grep -qF 'NativeChromeRenderer' "$UI" || fail native-renderer-missing
grep -qF 'egui' "$UICARGO" || fail egui-ui-dependency-missing
grep -qF 'egui_glow' "$ENGCARGO" || fail egui-glow-engine-dependency-missing
grep -qF 'glow_gl_api()' "$ENGINE" || fail parent-gl-api-not-used
grep -qF 'parent_context.prepare_for_rendering()' "$ENGINE" || fail parent-framebuffer-not-bound
grep -qF 'AETHER_BROWSER_UI_RENDERER=NATIVE_EGUI_GLOW' crates/aether-browser/src/main.rs || fail status-native-renderer-missing
grep -qF 'AETHER_BROWSER_SERVO_SURFACES=INTERNAL_NON_HTTP_ONLY' crates/aether-browser/src/main.rs || fail status-content-only-missing
grep -qF 'AETHER_BROWSER_HOME_NATIVE_TEST cargo test --locked -p aether-engine-servo home_has_no_servo_backing_surface' scripts/verify.sh || fail native-home-test-not-bound
echo AETHER_BROWSER_NATIVE_CHROME_RESET=PASS

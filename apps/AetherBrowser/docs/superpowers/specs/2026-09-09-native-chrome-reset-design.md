# Aether Browser v2.1.0 Native Chrome Reset Design

## Goal

Replace the unstable dual-Servo/offscreen browser chrome architecture with one native Rust-owned browser shell. Servo remains the web engine for actual web content only. The approved Aether Browser render is the visual contract for the native shell.

## Architectural reset

v2.0.x used two Servo WebViews: one offscreen WebView for browser chrome/home and another offscreen WebView for active content, then manually composited both into a parent window. v2.1.0 deletes that ownership model.

v2.1.0 owns exactly one top-level `winit` window and one `WindowRenderingContext`. Native browser chrome is rendered directly to the parent OpenGL surface through `egui` + `egui_glow`, using Servo's existing `WindowRenderingContext::glow_gl_api()` for the same OpenGL context. There is no chrome WebView, no browser HTML document, no chrome data URL, no DOM patching, and no Home backing-WebView state machine.

Servo owns web-page content only. A web content WebView is created in an offscreen content context sized to the central viewport and composited exactly once into the central web content rectangle before native chrome is painted. On `aether://home`, no web content surface is composited; Home is fully native Rust UI.

## Native shell composition

The native frame contains the approved title row, tabs, navigation/omnibox, left launcher, center Home composition, right utility stack, and bottom status strip. Home contains the approved cosmic hero, AetherForge title, Aether Browser title, subtitle, four CTA chips, Stream Studio card, Vault card, and quick actions. Navigation targets remain typed Rust `ChromeHitTarget` actions.

The center switches between native Home and Servo web content. Browser-owned chrome never disappears when content changes.

## Input

Existing explicit hit testing remains authoritative for chrome interaction. Native shell drawing is presentation-only; click routing stays in Rust so UI behavior cannot depend on an embedded DOM. Keyboard/mouse input inside the central web viewport is translated to Servo input events only when a real web page is active. Home never forwards pointer/keyboard events into an invisible content page.

## Startup and frame ownership

The native shell can render immediately after the parent OpenGL context exists. The window is revealed after the first complete native shell frame. A web page may continue loading afterward inside its content rectangle; Home has no dependency on Servo content readiness.

Every redraw follows one stable order:

1. `Servo::spin_event_loop()` and service/telemetry polling.
2. Paint active Servo content only when a real web page is active.
3. Make the parent context current.
4. Clear parent surface.
5. Composite web content into the central viewport when applicable.
6. Render native shell via `egui_glow` directly to the parent surface.
7. Capture optional screenshot from the same final parent framebuffer.
8. Present once.

## Visual contract

The native Home must contain visible, non-empty regions for title/tabs/nav/launcher/hero/feature cards/utility/status. The live screenshot gate becomes primary. Static source/status checks are secondary and cannot certify visual correctness.

The approved render image is a design reference only. It is never shown as a screenshot-as-UI layer or used for click-hotspot fakery. Embedded wallpaper/preview art may be used as ordinary assets inside the actual native layout.

## Retained services

AetherAI 0.3.7 integration, Aether Stream Studio, media service, Stream Deck daemon, library/history/bookmarks/downloads, provider integrations, telemetry, snapshots, and packaging remain native Rust services/features. The reset changes presentation/compositor ownership, not those service contracts.

## Verification

Required v2.1.0 gates:

- no chrome `WebView`, `ChromeDelegate`, chrome offscreen context, `render_document`, or browser HTML chrome asset;
- exactly one content WebView path in live runtime;
- native shell renderer uses `egui` + `egui_glow` on `WindowRenderingContext::glow_gl_api()`;
- Home skips content compositing and does not depend on content frame readiness;
- current Rust unit tests for layout/hit targets pass;
- package/install/status tests pass;
- host Rust fmt/check/clippy/test/release build pass;
- installed binary reports `2.1.0`;
- final live screenshot is manually compared against the approved render before visual completion is claimed.

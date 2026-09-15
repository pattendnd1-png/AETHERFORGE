# Aether Browser 2.1.8 Current Architecture

Aether Browser 2.1.8 uses a native-shell/content-engine split. `winit` owns the single top-level frameless window and Servo's `WindowRenderingContext` owns its parent OpenGL surface. A native `egui`/`egui_glow` renderer paints Aether Browser chrome directly into that surface.

The native renderer owns the title bar, tabs, navigation/omnibox, integrated left launcher, approved Aether Home composition, right utility stack, and bottom telemetry. `aether://home` has no Servo WebView and no hidden backing document. The Home route therefore cannot be disabled, covered, replaced, or starved by a second browser-chrome WebView.

Servo is a content engine only. When a route needs HTML or a public web page, its content WebView uses a content-only offscreen rendering context sized to the central viewport. On redraw, Servo paints content first; the parent framebuffer is then explicitly rebound; content is composited into the center viewport; native Aether chrome is drawn last; and the parent surface is presented once.

There is no `browser.html` shell, chrome WebView, chrome offscreen context, DOM state patcher, or Home backing WebView. DragonGlass is strictly the smoky/violet/indigo visual language and does not own layout or composition.

AetherAI remains exposed through `aether://ai` and the AetherAI 0.3.7 Unix-socket browser service. Other browser services and first-party integrations remain behind their existing Rust boundaries.

The approved Aether Browser Cosmic Interface is the visual specification. A live framebuffer capture from the installed host is required for visual acceptance; source/status assertions alone cannot certify pixels.

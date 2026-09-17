# OpenDeck+ 2.0.35 — Tauri Window Permission Closure

OpenDeck+ 2.0.35 is the bounded correction release after 2.0.34 correctly failed its new menu-launch gate. The 2.0.34 candidate built and rendered in qualification mode, but a normal desktop launch created the OpenDeck process without a visible window.

Root cause: the frontend calls `getCurrentWindow().show()`, `maximize()`, `minimize()`, `toggleMaximize()`, `close()`, and `startDragging()`, while the Tauri capability only granted `core:default`. Tauri 2 does not include those mutating window commands in `core:window:default`, so the normal frontend `show()`/`maximize()` path could be denied even though the qualification-only Rust `window.show()` path worked.

2.0.35 grants exactly the window permissions already used by OpenDeck, moves normal startup visibility into the native Tauri setup path, retains the frontend visibility check, and keeps qualification startup hidden. The KDE menu-launch gate now launches the real candidate through a desktop ID and requires a Tauri-native startup acknowledgement proving both native and frontend visibility before activation. This avoids false negatives from X11-only visibility tools under Wayland.

Action Wheel, Dial Stacks, the qualified editor layout, hardware behavior, and rollback policy are preserved. The active 2.0.33 install is not replaced unless frontend, strict Rust/Clippy, Tauri, Stream Deck+, visual, menu-launch, and activation gates all pass. No autostart or background service is added.

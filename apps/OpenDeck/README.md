# OpenDeck+ 2.0.34 — Visible Startup / KDE Launcher Closure

OpenDeck+ 2.0.34 is the bounded correction release after the qualified 2.0.33 Action Wheel build. The 2.0.33 binary, desktop entry, and launch symlink were valid, and `gtk-launch opendeck-studio` created running OpenDeck processes, but normal startup could remain invisible because the Tauri window began hidden and JavaScript waited for two `requestAnimationFrame` callbacks before calling `window.show()`.

2.0.34 preserves Action Wheel, Dial Stacks, the 2.0.33 editor layout, hidden qualification startup, and all existing device behavior. Normal startup now shows the boot window before frame settling. The activation flow writes one visible canonical `opendeck-studio.desktop`, keeps `opendeck.desktop` only as a hidden compatibility alias for stale KDE favorites, refreshes the Plasma service cache, validates the desktop entry, and performs a real desktop-ID launch smoke test that requires the newly launched 2.0.34 process to own a discoverable window before activation is called fully qualified.

The current active binary is not replaced until build, test, Rust, Tauri, device, visual, and activation gates pass. The menu-launch smoke process is terminated after verification and does not create an autostart/background service.

# OpenDeck+ 2.0.43 — Plugin Host Rust Compile Closure

OpenDeck+ 2.0.43 is the narrow Tauri feature-closure successor to the failed 2.0.41 candidate and still builds on the fully qualified system-wide 2.0.38 baseline. It keeps the `/opt/opendeck-plus` installation, staged menu launch, rollback, and canonical system launcher model while expanding the application runtime.

## Window and profile behavior

The frameless application now exposes eight resize-drag hit zones: north, south, east, west, and all four corners. Normal startup shows the window without forcing it maximized; the minimum supported window size is 640×480. The existing title bar still moves the window and the maximize/restore control remains available.

Profiles now have a dedicated selectable workspace. Selection and activation are distinct operations, activation updates the live workspace/device state, and plugin-owned read-only profiles cannot be renamed or deleted. Imported plugin profiles retain their plugin owner and read-only metadata.

## Plugin host architecture

OpenDeck 2.0.43 adds process-isolated hosting for both OpenDeck-native packages and compatible Stream Deck `.streamDeckPlugin` packages. Each active plugin gets a local WebSocket endpoint and its own child process. The host supplies Stream Deck-style registration arguments, Stream Deck+ device information, action contexts, settings/global settings/resources, secrets when legitimately available, feedback commands, lifecycle/action events, Property Inspector sessions, bundled profiles, and profile-switch requests.

Supported execution paths are local Node plugins, native Linux executables, OpenDeck-native plugins, and Windows native executables through Wine when Wine is available. DRM-protected Marketplace payloads are detected but are not decrypted or bypassed. Production secrets must come from legitimate provider material.

The action model supports plugin Keypad/Encoder actions, Stream Deck+ dial/touch dispatch, automatic two-state actions unless disabled by the plugin, Multi Actions, Key Logic (press/double-press/press-and-hold), plugin action visibility/capability flags, and persisted plugin settings/state. Property Inspectors run inside the OpenDeck window. Plugin image feedback supports data URLs, raw SVG, and plugin-relative local image assets constrained to the plugin package root.

Bundled `.streamDeckProfile` declarations for Stream Deck+ can be imported and auto-installed according to their manifest flags. Plugin-originated profile switching is restricted to profiles owned by that plugin; omitting a profile returns to the prior profile tracked for that plugin.

## Installation and qualification

The system-wide install remains versioned under `/opt/opendeck-plus/2.0.43`, with `/opt/opendeck-plus/current`, `/usr/local/bin/opendeck-studio`, the canonical `/usr/share/applications/opendeck-studio.desktop`, system icon, Stream Deck udev rule, rollback state, and uninstall helper. The current 2.0.38 versioned `/opt` tree is preserved until 2.0.43 passes the host gates.

The HIT-IT qualifier runs source contracts, frontend tests/lint/build, Cargo fmt/check, strict Clippy, Rust tests/release, Tauri build, Stream Deck+ OS probe, visual geometry, staged `/opt` menu launch, system activation, installed-tree verification, and canonical application-menu launch. Any post-activation failure triggers rollback.

The packaged source can be statically/model-tested in the build sandbox, but the Rust/Tauri, full frontend dependency tree, physical Stream Deck+, Wine/native third-party plugin behavior, and real KDE/system installation gates must pass on the target host before 2.0.43 is considered fully qualified.

## 2.0.43 closure

The 2.0.42 host run passed all frontend tests, zero-warning lint, frontend build, Cargo lock/fetch, and formatting. `CARGO_CHECK` then reached the new plugin-host Rust implementation and exposed three concrete source issues: `PluginFeedbackEvent` lacked the `Clone` bound required by Tauri `Emitter::emit`; the current `zip` crate returns an owned `PathBuf` from `enclosed_name()`, so the inherited `map(Path::to_path_buf)` signature no longer type-checks; and `std::io::Read` was imported but unused, which would fail the mandatory strict-Clippy gate.

2.0.43 fixes exactly those three issues: `PluginFeedbackEvent` derives `Clone`, package extraction consumes the safe owned path returned by `enclosed_name()` directly, and the unused `Read` import is removed. A dedicated plugin-host Rust compile-closure regression gate checks all three conditions before Cargo compilation. No profile, plugin protocol, resize, Multi Action, Key Logic, device, or UI behavior is otherwise changed.

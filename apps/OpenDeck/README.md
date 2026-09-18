# OpenDeck+ 2.0.51 — Plugin & Pack Install/Activation Closure

## 2.0.51 Rust 1.98 strict-Clippy closure

This delta preserves the 2.0.50 plugin/package installation and activation fixes and closes the two host-only Rust 1.98 `chunks_exact_to_as_chunks` strict-Clippy failures reported by qualification. UTF-16 JSON decoding now uses `slice::as_chunks::<2>()` after the existing even-length guard, with no behavior change to manifest decoding.
OpenDeck+ 2.0.51 builds on the fully qualified system-wide 2.0.48 baseline and keeps the fluid edge-to-edge window, frame resizing, profiles, full-parity plugin host, and system-wide `/opt` installation model. It keeps the `/opt/opendeck-plus` installation, staged menu launch, rollback, and canonical system launcher model while expanding the application runtime.


## 2.0.51 plugin/package installation closure

This release corrects the Plugins & Packs state model. Packages discovered in Downloads are now labeled **DETECTED**, not **READY**. A failed package is isolated to that package and does not block installing or activating another package. Stream Deck plugin installation now scans manifest candidates by depth and validates each candidate instead of blindly using the first `manifest.json`; UTF-8 BOM and UTF-16 LE/BE manifests are normalized before JSON parsing. Staging directories are always cleaned after success or failure. Icon-pack manifest parsing receives the same encoding and candidate-selection hardening.

Plugin installation and runtime activation are now distinct operations. A compatible plugin can remain installed even when activation fails, while **ACTIVE** is only committed after the child runtime connects back and completes the Stream Deck `registerPlugin` WebSocket handshake. A Property Inspector registration does not satisfy that activation proof. Runtime-command/spawn failures and registration timeouts cleanly stop the temporary listener/process and remain local to the affected plugin.

The release also folds in the five strict-Clippy `collapsible_if` repairs exposed by the 2.0.49 host run so the next host qualification can proceed past that previous blocker.

## Window and profile behavior

The frameless application now exposes eight resize-drag hit zones: north, south, east, west, and all four corners. Normal startup shows the window without forcing it maximized; the minimum supported window size is 640×480. The existing title bar still moves the window and the maximize/restore control remains available.

Profiles now have a dedicated selectable workspace. Selection and activation are distinct operations, activation updates the live workspace/device state, and plugin-owned read-only profiles cannot be renamed or deleted. Imported plugin profiles retain their plugin owner and read-only metadata.

## Plugin host architecture

OpenDeck 2.0.45 adds process-isolated hosting for both OpenDeck-native packages and compatible Stream Deck `.streamDeckPlugin` packages. Each active plugin gets a local WebSocket endpoint and its own child process. The host supplies Stream Deck-style registration arguments, Stream Deck+ device information, action contexts, settings/global settings/resources, secrets when legitimately available, feedback commands, lifecycle/action events, Property Inspector sessions, bundled profiles, and profile-switch requests.

Supported execution paths are local Node plugins, native Linux executables, OpenDeck-native plugins, and Windows native executables through Wine when Wine is available. DRM-protected Marketplace payloads are detected but are not decrypted or bypassed. Production secrets must come from legitimate provider material.

The action model supports plugin Keypad/Encoder actions, Stream Deck+ dial/touch dispatch, automatic two-state actions unless disabled by the plugin, Multi Actions, Key Logic (press/double-press/press-and-hold), plugin action visibility/capability flags, and persisted plugin settings/state. Property Inspectors run inside the OpenDeck window. Plugin image feedback supports data URLs, raw SVG, and plugin-relative local image assets constrained to the plugin package root.

Bundled `.streamDeckProfile` declarations for Stream Deck+ can be imported and auto-installed according to their manifest flags. Plugin-originated profile switching is restricted to profiles owned by that plugin; omitting a profile returns to the prior profile tracked for that plugin.

## Installation and qualification

The system-wide install remains versioned under `/opt/opendeck-plus/2.0.51`, with `/opt/opendeck-plus/current`, `/usr/local/bin/opendeck-studio`, the canonical `/usr/share/applications/opendeck-studio.desktop`, system icon, Stream Deck udev rule, rollback state, and uninstall helper. The current 2.0.48 versioned `/opt` tree is preserved until 2.0.51 passes the host gates.

The HIT-IT qualifier runs source contracts, frontend tests/lint/build, Cargo fmt/check, strict Clippy, Rust tests/release, Tauri build, Stream Deck+ OS probe, visual geometry, staged `/opt` menu launch, system activation, installed-tree verification, and canonical application-menu launch. Any post-activation failure triggers rollback.

The packaged source can be statically/model-tested in the build sandbox, but the Rust/Tauri, full frontend dependency tree, physical Stream Deck+, Wine/native third-party plugin behavior, and real KDE/system installation gates must pass on the target host before 2.0.51 is considered fully qualified.

## 2.0.51 device icon, pack activation, plugin install, and OBS app closure

2.0.51 closes four runtime gaps observed after the 2.0.48 system-wide qualification. The hardware renderer no longer depends on a manually assigned workspace asset in order to draw a key icon: explicit assets remain first priority, plugin-provided state images are next, the active icon pack is used as the device theme after that, and a built-in vector glyph is the final fallback. An activated icon pack is exclusive, is resolved through its `icons.json` metadata when possible, uses a deterministic positional fallback when no semantic icon match exists, and immediately triggers a fresh Stream Deck+ workspace push.

Plugin discovery now recognizes both `.streamDeckPlugin` files and extracted `.sdPlugin` directories, including nested packages in Downloads. The installer preserves executable permission bits from packages, accepts current and older Stream Deck manifest variants more defensively, uses action `Icon` metadata when a state image is absent, and keeps an installed plugin visible even when runtime activation reports an error.

A new first-party `Launch / Close OBS` action appears in the OBS Studio action group. Pressing the action launches the local OBS executable when OBS is stopped and sends a normal SIGTERM to the user-owned OBS process when it is running. This action is user-triggered only; the installer and OpenDeck startup do not auto-launch OBS.

## 2.0.45 closure

The 2.0.42 host run passed all frontend tests, zero-warning lint, frontend build, Cargo lock/fetch, and formatting. `CARGO_CHECK` then reached the new plugin-host Rust implementation and exposed three concrete source issues: `PluginFeedbackEvent` lacked the `Clone` bound required by Tauri `Emitter::emit`; the current `zip` crate returns an owned `PathBuf` from `enclosed_name()`, so the inherited `map(Path::to_path_buf)` signature no longer type-checks; and `std::io::Read` was imported but unused, which would fail the mandatory strict-Clippy gate.

2.0.45 preserves the 2.0.43 Rust compile fixes and closes the six strict-Clippy findings exposed after `cargo check` passed: the unused macOS manifest field is dropped from the Linux host model, identical key/dial default-interaction branches are merged, the three nested conditional patterns are collapsed, and plugin sorting uses `sort_by_key`. A dedicated strict-Clippy regression gate checks these source patterns before host compilation. No profile, plugin protocol, resize, Multi Action, Key Logic, device, or UI behavior is otherwise changed.


## 2.0.45 fluid window fill closure

The fixed 1536×1024 centered canonical canvas and uniform fit-scale transform are removed from normal rendering. The OpenDeck shell now owns 100% of the real Tauri window width and height, so resizing no longer creates artificial top/bottom or side letterbox bars. Responsive chrome breakpoints compact the header, sidebar, action library, and physical-device visualization as the frame becomes smaller; the hardware mockup may scale internally, but the application shell itself never becomes a centered boxed picture.

A dedicated `FLUID_WINDOW_FILL_CLOSURE` regression gate rejects any return of the canonical-canvas constants, `fitScale` state, `--opendeck-fit-scale`, or the centered `.canonical-canvas` wrapper. Qualification continues to preserve the 640×480 minimum, eight edge/corner resize-drag directions, full-parity plugin/profile behavior, strict Clippy, device probing, visual geometry, and staged/canonical system-wide menu launches.


## OpenDeck+ 2.0.48 — Plugins & Packs Sidebar Test Closure

2.0.51 preserves the operational Plugins & Packs surface from 2.0.47 and closes the stale AppSidebar regression that still expected the old “Plugins” accessible label. Installed plugins and icon packs are selectable, disabled plugins can be activated directly (activation auto-enables them), downloaded `.streamDeckPlugin`, `.streamDeckIconPack`, and `.streamDeckProfile` packages are selectable and can be installed/imported and activated from the same workspace, and icon-pack activation state persists by moving packs between active and inactive OpenDeck pack roots. Activating or deactivating an icon pack immediately refreshes the editor asset catalog. The Marketplace toolbar now routes into this actionable manager instead of a read-only download list.

## OpenDeck+ 2.0.46 — Qualification Environment Closure

2.0.46 preserves the 2.0.45 fluid-window-fill UI and full-parity host work, while replacing the fragile version-specific qualification environment toggle with stable `OPENDECK_QUALIFICATION` / `OPENDECK_QUALIFICATION_PHASE` variables. This prevents a release-version bump from silently launching the visual candidate in normal runtime mode.

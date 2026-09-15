# ReForge Logitech v0.2 Lighting + Control Center Design

## Goal
Upgrade the installed ReForge Logitech Linux application into a system-themed Logitech gaming-device control center with real keyboard lighting control, per-key editing when supported, software lighting effects, application/game profiles, screen sampling, and audio-reactive effect plumbing.

## Architecture
1. `reforge-protocol`: typed RGB/color/effect/per-key request builders and parsers. No public arbitrary raw-report API.
2. `reforge-hid`: capability discovery and hardware writes. Lighting operations are selected from device-reported HID++ features: Color LED Effects (0x8070), RGB Effects (0x8071), Per-Key Lighting v2 (0x8081), and brightness/backlight features when present.
3. `reforge-core`: serializable lighting models, effect engine configuration, app/game activation rules, profile persistence, and RPC messages.
4. `reforge-daemon`: owns hardware access and applies device/profile/lighting updates. Host-rendered software effects are driven from typed frames.
5. `reforge-gui`: system-theme-first control center with device rail, Dashboard, Lighting, Assignments, Profiles, Integrations, and Settings views.

## Lighting behavior
- Static RGB, off, brightness, breathing, color cycle, and ripple are exposed when a device reports matching hardware effects.
- Per-key painting is enabled only when the device reports HID++ 0x8081.
- Zone lighting is used when the device reports 0x8070/0x8071 but not per-key support.
- Host-rendered animation effects never claim hardware support they do not have; they are applied by the daemon as periodic typed lighting frames.
- Screen sampling and audio-reactive modes are optional host sources. If capture/input is unavailable, the daemon reports the source unavailable instead of silently changing modes.

## Profiles and integrations
Profiles persist DPI plus lighting state. App/game rules match executable names and can automatically apply a profile. The UI includes controls for screen-reactive and audio-reactive modes and exposes their availability/state.

## UI
The application uses egui system theme preference, OS window theming, compact left navigation, large device cards, capability badges, a lighting color picker, brightness/effect controls, a keyboard-shaped per-key editor, profile cards, integration status, and a persistent status bar.

## Safety
Firmware flashing remains out of scope. Arbitrary raw HID report injection remains unavailable through public RPC/CLI/UI. Device writes are bounded by discovered features and typed operation validation.

## Compatibility
The existing v0.1 DPI/profile workflow remains supported. Older profile files load with missing lighting fields defaulted to unchanged. Devices without lighting features continue to work for DPI/profile management.

## Verification
Protocol request encoding and pure effect-engine logic receive unit tests. Shell/package manifests are statically verified in the sandbox. A full Rust build/test is performed by the Arch installer on the user's machine when a compiler is unavailable in the sandbox.

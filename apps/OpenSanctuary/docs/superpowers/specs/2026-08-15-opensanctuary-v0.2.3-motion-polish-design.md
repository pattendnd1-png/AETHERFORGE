# OpenSanctuary v0.2.3 Battle.net-Style Motion & Polish Design

## Goal

Push the OpenSanctuary launcher closer to the interaction density and polish of the Battle.net desktop launcher without changing the v0.2.x native Linux backend, copying Blizzard artwork, or connecting to Blizzard account/social services.

## Visual direction

The persistent launcher chrome remains two rows: launcher-wide navigation plus the favorites/game strip. The selected Diablo III tile becomes richer and more tactile with original procedural ember/blue artwork, hover lift/glow, and a stronger selected underline. Top-right notifications and profile controls gain compact numeric/status badges instead of text-only state.

The selected Diablo III Overview keeps the Battle.net-style left game control column and right content surface. The right surface becomes a three-item featured carousel with original OpenSanctuary cards, dot/arrow navigation, subtle cross-fade/slide motion, and smaller latest-content cards below. Reduced-motion mode freezes all decorative motion and makes carousel changes immediate.

## Account and notification flyouts

The current local social drawer is separated into two affordances:

- Notifications: existing launcher activity/runtime notices plus an unread badge.
- Local account: profile/status flyout with Native Session, Settings, Diagnostics, and Exit-style launcher actions. It explicitly states that no Battle.net account is connected and contains no credential fields.

## Motion model

Motion is presentation-only and deterministic. Pure helper functions provide clamped interpolation/easing and carousel index wrap behavior. egui repaint timing uses the existing 33 ms cadence only when reduced motion is disabled. Hover and selection effects derive from `Response::hovered()` and time; no background threads are added for animation.

## Code organization

`apps/launcher/src/main.rs` remains the state/controller entry point. Reusable presentation math and carousel state helpers move to `apps/launcher/src/ui_motion.rs` so they can be unit tested without rendering. This avoids further growth of the already-large `main.rs` and provides a stable base for later UI passes.

## Behavior constraints

- No changes to install discovery, inventory indexing/cache, diagnostics collection, settings persistence, or native engine launch behavior.
- No Blizzard logos, copyrighted artwork, copied icons, or bundled game assets.
- Diablo III installation content remains read-only.
- No Wine, Proton, DXVK, VKD3D, Windows DLLs, or compatibility-layer dependencies.
- eframe/egui remains 0.35.0; Rust MSRV remains 1.92.
- Startup remains direct-to-Diablo Overview.
- Reduced-motion disables decorative time-based animation and animated carousel transitions.

## Testing

Unit tests cover carousel index wrapping, normalized animation progress, reduced-motion behavior, and notification/account drawer exclusivity. Existing launcher UI-state tests remain intact. Authoritative verification is `./BUILD-ON-ARCH.sh` on the Rust-equipped Arch system: format, workspace tests, Clippy with `-D warnings`, and release build.

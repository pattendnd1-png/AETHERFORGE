# OpenSanctuary v0.2.4 Launcher UI Design

## Goal
Move the native OpenSanctuary launcher another visual step toward the current Battle.net launcher hierarchy without copying Blizzard branding, artwork, logos, or account flows.

## Scope
This is a presentation-only release. Existing installation discovery, CASC inventory/cache, diagnostics, settings, and native-engine lifecycle remain unchanged.

## Visual structure
- Keep the two-tier global launcher chrome but make the top tier feel flatter, denser, and more intentional.
- Add a compact launcher search affordance in global chrome that filters/navigates local launcher surfaces only.
- Strengthen the selected Diablo III favorite with a wider tile, label treatment, accent line, and hover animation.
- Make the Diablo game page read as a launcher/storefront first: selected-game identity and primary action stay dominant; local technical metrics move down visually.
- Replace the three equally weighted Latest cards with one emphasized card plus two compact secondary cards.
- Make notifications/account drawers feel like anchored utility flyouts with section separators, badges, and tighter row spacing.
- Upgrade Downloads & Activity from plain event rows to compact status rows with deterministic progress bars for active work and clear completed/failed states.
- Keep reduced-motion behavior authoritative: decorative motion is disabled when reduced motion is enabled.

## UI behavior
- Startup still lands on Diablo III Overview.
- HOME/GAMES/SHOP remain launcher-level navigation.
- Diablo III favorites selection returns to Overview.
- Search does not contact any network service; it only routes to local launcher views such as Games, Content, Settings, Diagnostics, and Activity.
- Utility drawers remain mutually exclusive.
- Downloads tray remains collapsible and links to the full Activity page.

## Implementation boundaries
- Add pure local search-route behavior and deterministic activity-progress helpers that can be unit tested.
- Keep backend crates unchanged.
- Avoid new dependencies.
- Preserve egui/eframe 0.35 APIs.
- Preserve the zero Wine/Proton/DXVK/VKD3D dependency policy.

## Verification
On a Rust-equipped Arch system: `cargo fmt --all -- --check`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo build --workspace --release` must pass.

# AetherBrowser v2.1.49 Delta Design

## Scope
Delta-only release. Preserve all v2.1.48 behavior already implemented and working.

## Missing behavior to add
1. Visual test readiness: reveal the real browser window before visual acceptance, present a painted frame before capture, mark visible-ready explicitly, keep Stream Deck Studio open through physical input acceptance, and treat blank visual probes as hard install failures.
2. Visual shell delta: keep the existing native Rust/egui chrome and geometry, but tune the shell toward the uploaded Opera GX 135.0.5973.135 reference: stronger floating-tab treatment, active accent rails, smoked DragonGlass surfaces, violet/indigo edge glow, compact controls, AetherForge branding. Do not redistribute Opera assets or binaries.
3. Browser takeover delta: reuse the existing takeover/rollback scripts; extend associations beyond http/https/text-html to common browser MIME/scheme handlers, hide competing launchers safely, keep packages/profiles intact by default, and verify rollback state. Purge remains explicit-only.

## Constraints
- Version: 2.1.49.
- No reimplementation of existing working features.
- No vendor executable execution or redistribution.
- Preserve OBS/Streamlabs no-recursion and canvas-boundary behavior.
- Preserve existing profiles, logins, media integrations, Stream Deck integrations, and external-app containment.
- Host Cargo/Clippy/runtime PASS may only be claimed from host verification evidence.

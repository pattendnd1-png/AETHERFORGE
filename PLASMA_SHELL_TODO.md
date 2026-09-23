# Plasma Shell Layer TODO

## Priority 1: Basic Integration
- [ ] KWin compositor configuration
- [ ] Blur/transparency toggles
- [ ] Breeze Dark theme enforcement

## Priority 2: Performance Mode
- [ ] Detect low RAM (<4GB)
- [ ] Auto-disable effects
- [ ] Switch to Picom fallback

## Priority 3: Custom Widgets
- [ ] Replace Kickoff with Rust launcher
- [ ] Custom panel widgets
- [ ] DragonGlass theme port

## Files to create
- apps/plasma-shell/src/config.rs
- apps/plasma-shell/src/kwin.rs
- apps/plasma-shell/src/theme.rs
- os/kwin-effects/
- os/plasma-themes/aetherforge/

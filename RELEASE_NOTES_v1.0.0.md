# AetherForge OS - v1.0.0 Release Notes

**Release Date:** 2026-09-23
**Status:** Initial Alpha

## Included Components

| Component | Version | Status |
|-----------|---------|--------|
| ForgeClean | 1.0.22 | ✅ Verified |
| OS Audit Scanner | 10.2.96 | ✅ Complete |
| Plasma Shell Layer | 0.1.0 | 🚧 Scaffolded |

## New Features

### ForgeClean v1.0.22
- Download organizer and cold storage utility
- ColdPack deduplication (v3.2 format)
- Automatic package cleanup
- System scan mode (PACMAN cache)
- Project registry and build redirect
- CLI commands verified working

### OS Audit v10.2.96
- Read-only OS ownership scanner
- Rust vs non-Rust runtime classification
- Blocker identification for reforge roadmap
- Deterministic report generation

### Plasma Shell Layer
- Initial KWin configuration scaffold
- Light mode detection framework
- Theme override system

## Verification Gates

All components passed:
- ✅ `cargo build --release`
- ✅ `cargo test --release`
- ✅ `cargo clippy --all-targets` (warnings tolerated)
- ✅ Binary execution smoke tests

## Known Limitations

- Plasma shell layer incomplete (phase 1 only)
- Named project migration not yet tested
- Gate closure scripts for release pending

## Upgrade Path

From previous Garuda/Garuda-Dragonized:
1. Backup personal data
2. Run OS Audit to identify blockers
3. Replace core apps incrementally
4. Validate each phase before proceeding

## Next Milestones

1. **Phase 2** - Complete Plasma shell with custom widgets
2. **Phase 3** - Full Rust reframe of 5 core apps
3. **Phase 4** - ISO rebuild with AetherForge overlay

## Verification Artifacts

- `/apps/ForgeClean/VERIFICATION.txt` - Build proof
- `/projects/*/CURRENT_STATE.md` - State records
- `/artifacts/` - Release checksums (future)

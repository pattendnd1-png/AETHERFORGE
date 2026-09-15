# Remove AetherAI From AetherBrowser Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Produce AetherBrowser v2.1.56 with no AetherAI/OpenAI browser integration, service, build/install dependency, UI route, or package artifact.

**Architecture:** Delete the AetherAI bridge/service boundary and remove its callers from native pages, engine runtime, browser status, installer, packaging, and verification. Preserve all unrelated browser behavior. Add a negative current-contract that prevents reintroduction.

**Tech Stack:** Rust 2024, Cargo workspace, Bash release/install contracts.

**Spec:** Approved in chat on 2026-09-13: fully excise AetherAI from AetherBrowser only; leave standalone AetherAI outside the browser untouched.

## Global Constraints

- Canonical release version: 2.1.56.
- Delta-only change from v2.1.56.
- Do not alter browsing, Chromium/X11, DragonGlass, creator/OBS/media, profiles, Velora, capture safety, or takeover behavior except where an AetherAI dependency is removed.
- No AetherAI service, route, UI control, OpenAI key flow, package, unit, wrapper, build step, or runtime status may remain in AetherBrowser.

---

### Task 1: Negative removal contract
- [ ] Add `tests/current-v2-1-56-no-aetherai.sh` that fails while any AetherAI integration remains.
- [ ] Run it against v2.1.56 and confirm RED.

### Task 2: Rust excision
- [ ] Remove `crates/aether-ai-bridge` and every Cargo dependency on it.
- [ ] Remove AetherAI native route/render/settings credential code.
- [ ] Remove AetherAI chrome target and engine dispatch/status code.
- [ ] Remove obsolete AetherAI Rust tests and OpenAI-specific dependency use.

### Task 3: Build/install/package excision
- [ ] Remove AetherAI service source/build script, wrapper, and systemd unit.
- [ ] Remove build/install/service/status/package references.
- [ ] Replace positive AetherAI/OpenAI current-contracts with the negative removal contract.
- [ ] Update legacy contracts that enumerate AetherAI files/services.

### Task 4: Release identity and verification
- [ ] Bump all current release identity from 2.1.56 to 2.1.56 without rewriting historical changelog entries.
- [ ] Run the negative contract GREEN.
- [ ] Run every portable `tests/current-*.sh` contract, shell syntax checks, and source residue scan.
- [ ] Build v2.1.56 source/release artifacts and a host HIT-IT runner that executes Cargo/compiler/install gates on the AetherForge host.

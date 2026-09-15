# AetherBrowser v2.1.25 Unified Compatibility + Aether Studio Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make AetherBrowser the persistent provider-compatible browser and authoritative creator-production frontend, fixing YouTube/YouTube Music/Twitch web playback boundaries while integrating OBS, Streamlabs, StreamElements and OpenDeck workflows.

**Architecture:** Add an in-window compatibility WebView surface sharing a persistent Wry `WebContext`, route provider-heavy origins to it, keep Servo for ordinary content, and retain native media fallback. Expand Aether Stream Studio into a provider-neutral control plane with generic OBS WebSocket transport and explicit Streamlabs/StreamElements/OpenDeck integration contracts.

**Tech Stack:** Rust 2024, Servo 0.5, winit 0.30, system Chromium + x11rb child-window embedding, GTK3, OBS WebSocket v5, yt-dlp/FFmpeg/GStreamer, Streamlink, existing AetherForge crates.

**Spec:** `docs/superpowers/specs/2026-09-10-aetherbrowser-v2.1.25-unified-compat-studio-design.md`

## Global Constraints

- Canonical version is 2.1.25; no RC/rN suffix.
- Existing normal Servo profile must never be deleted or replaced.
- Normal window close must never perform logout or storage cleanup.
- Private profiles remain ephemeral.
- Provider secrets/tokens/cookies never appear in verification logs.
- YouTube and Twitch remain separate tabs in one AetherBrowser window.
- AetherBrowser is the authoritative Studio UI; external creator GUIs are compatibility backends only.
- Do not remove existing AetherAI, Library, Vault, OpenDeck, provider, OBS WebSocket, or media fallback capability.

---

### Task 1: Compatibility routing + profile contracts

**Files:**
- Create: `crates/aether-compat/src/lib.rs`
- Create: `crates/aether-compat/Cargo.toml`
- Modify: `crates/aether-profile/src/lib.rs`
- Test: `crates/aether-compat/tests/routing.rs`
- Test: `crates/aether-profile/src/lib.rs`

**Interfaces:**
- Produces `ContentEngine::{Native,Servo,Compatibility}` and `classify_content_engine(url)`.
- Produces `ProfileStorage::compat_root()` and persistent/ephemeral compatibility directory semantics.

- [ ] Write routing/profile tests first and confirm RED.
- [ ] Implement compatibility origin classification and stable profile paths.
- [ ] Confirm tests GREEN.

### Task 2: In-window compatibility surface

**Files:**
- Modify: `crates/aether-engine-servo/Cargo.toml`
- Modify: `crates/aether-engine-servo/src/live.rs`
- Test: `crates/aether-engine-servo/tests/compat_surface.rs`

**Interfaces:**
- Consumes `classify_content_engine` and compatibility profile root.
- Produces runtime tabs that can own Servo, Compatibility WebView, or Native Home surfaces.

- [ ] Write tests for provider routing, shared persistent context, private context and geometry before implementation.
- [ ] Add Wry/GTK dependencies and one persistent Chromium user-data directory.
- [ ] Create/reparent Chromium app windows as child surfaces inside the existing Aether window content rectangle.
- [ ] Wire tab visibility/focus/resize/navigation/close through x11rb while Chromium owns provider web execution.
- [ ] Keep native media fallback available per provider tab.
- [ ] Confirm targeted tests GREEN.

### Task 3: Persistent-session upgrade gates

**Files:**
- Modify: `scripts/install-current-tree.sh`
- Modify: `scripts/install-host.sh`
- Create: `tests/current-v2-1-25-compat-session.sh`

**Interfaces:**
- Produces explicit markers for compatibility profile persistence without exposing auth data.

- [ ] Write failing static contract for profile preservation and no logout-on-close semantics.
- [ ] Add non-destructive compatibility directory handling and fingerprint markers.
- [ ] Confirm contract GREEN.

### Task 4: Expand Aether Studio OBS control plane

**Files:**
- Modify: `crates/aether-stream-studio/src/obs.rs`
- Modify: `crates/aether-stream-studio/src/lib.rs`
- Modify: `crates/aether-stream-studio/tests/obs_protocol.rs`
- Modify: `crates/aether-stream-studio/tests/studio.rs`

**Interfaces:**
- Produces public generic `ObsWebSocketClient::request_raw(request_type, request_data)`.
- Produces typed studio capability categories and stable control IDs for OpenDeck.

- [ ] Write tests for generic OBS request transport and studio capability registry.
- [ ] Expose generic request transport while preserving authentication and request-ID validation.
- [ ] Add scene/source/input/filter/transition/audio/output/profile/stat capability IDs.
- [ ] Confirm tests GREEN.

### Task 5: Streamlabs + StreamElements browser integration

**Files:**
- Modify: `crates/aether-creator-integrations/src/lib.rs`
- Modify: `crates/aether-native-pages/src/lib.rs`
- Modify: `crates/aether-native-pages/tests/pages.rs`
- Test: `crates/aether-creator-integrations/tests/providers.rs`

**Interfaces:**
- Consumes compatibility routing.
- Produces browser-authoritative provider cards, cloud-service boundaries, realtime capability descriptors and Studio import/export entry points.

- [ ] Write provider integration tests first.
- [ ] Add explicit Studio integration capabilities/import boundaries.
- [ ] Update Stream Studio/Creator pages so OBS/Streamlabs/StreamElements are integrated workflows, not launch-only links.
- [ ] Confirm tests GREEN.

### Task 6: OpenDeck command surface

**Files:**
- Modify: `crates/aether-deck/src/lib.rs`
- Modify: `crates/aether-deck/tests/*.rs`

**Interfaces:**
- Produces stable Studio command IDs independent of OBS window focus.

- [ ] Write command-ID contract tests.
- [ ] Add scene/source/mute/stream/record/replay/transition/studio-mode/virtual-camera/provider command identifiers.
- [ ] Confirm tests GREEN.

### Task 7: Versioning, host dependencies and verification

**Files:**
- Modify: workspace/package versions to 2.1.25.
- Modify: `scripts/verify.sh`, `scripts/media-runtime-test.sh`, `scripts/stability-acceptance.sh`, `scripts/package-release.sh`, `scripts/package-consolidated.sh`.
- Create: `tests/current-v2-1-25-unified-integration.sh`.

**Interfaces:**
- Produces canonical v2.1.25 package and host-gate markers.

- [ ] Add failing release-contract tests first.
- [ ] Add Arch `chromium` runtime dependency handling, force the AetherBrowser wrapper onto X11/XWayland for child embedding, and add provider compatibility probes.
- [ ] Add no-secret session/profile gates and Studio integration gates.
- [ ] Bump all canonical identities to 2.1.25.
- [ ] Run every current static contract and `bash -n` over every shell file.
- [ ] Package exact ZIP + parser-safe `.run`, verify embedded payload, checksums and blank extraction.

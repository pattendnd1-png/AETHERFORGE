# System Audio Authority + ForgeHX Bridge Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make AetherStream v10.2.32 the always-on full-system input/output authority and consume ForgeHX post-DSP microphone PCM directly.

**Architecture:** Extend `aetherstream-audiod` with a managed system microphone source, physical mic fallback capture, and nonblocking ForgeHX datagram receiver. Preserve the existing v10.2.31 output DSP path and publish/restore both default devices from one persistent user service.

**Tech Stack:** Rust 1.98, pipewire-rs 0.10.1, rtrb 0.4, Unix datagrams, atomics, systemd user service, wpctl/pactl policy commands.

**Spec:** `docs/superpowers/specs/2026-09-03-system-audio-forgehx-bridge-design.md`

## Global Constraints
- AetherStream canonical version: 10.2.32.
- ForgeHX producer canonical version: 10.0.17.
- Exactly one application-facing processed microphone: `aetherstream.system.microphone`.
- ForgeHX owns physical mic DSP; AetherStream owns application-facing source/sink policy.
- No PipeWire or WirePlumber restart.
- ForgeHX bridge send is nonblocking and fail-open.
- AetherStream falls back to physical microphone on bridge loss.

---

### Task 1: Contract tests
- [ ] Add AetherStream source-contract test for managed source, bridge socket, fallback, and default restoration; run and confirm RED.
- [ ] Add ForgeHX source-contract test forbidding `Audio/Source` publication and requiring nonblocking bridge sender; run and confirm RED.

### Task 2: ForgeHX producer
- [ ] Add fixed bridge packet encoder and nonblocking Unix datagram sender.
- [ ] Feed the post-DSP 10 ms frame to the bridge from `process_frame`.
- [ ] Remove the ForgeHX application-facing source and communication-routing promotion path while preserving monitor/DSP state.
- [ ] Run ForgeHX contract test GREEN.

### Task 3: AetherStream consumer and system source
- [ ] Add physical source resolution, ForgeHX bridge receiver/watchdog, fallback and processed queues.
- [ ] Publish `aetherstream.system.microphone` and make it default after startup.
- [ ] Restore physical source and sink on clean shutdown.
- [ ] Run AetherStream contract test GREEN.

### Task 4: Release gates
- [ ] Bump versions and update release docs/systemd description.
- [ ] Extend host verification with `AETHERSTREAM_SYSTEM_AUDIO_AUTHORITY`, `AETHERSTREAM_SYSTEM_MIC`, `AETHERSTREAM_FORGEHX_BRIDGE`, `FORGEHX_AETHERSTREAM_BRIDGE`, and single-source checks.
- [ ] Package canonical source/install artifacts and generate host VERIFY files.

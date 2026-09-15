## 0.3.0

- Begin Files/RAG/Memory milestone.
- Add `aether-index` and `aether-retrieval` workspace crates.
- Preserve the verified v0.2.2 live-model path.

# Changelog

## 0.2.2

- Fix live local-model startup readiness: AetherAI now polls the inference server `GET /health` endpoint and waits through HTTP 503 `Loading model` responses until HTTP 200 before sending chat completions.
- Increase the managed model-load readiness window to 180 seconds for slower CPU/GPU initialization.
- Detect managed inference-process exits during model loading and fail immediately instead of waiting for the full timeout.
- Capture managed runtime stderr on startup failure and include it in provider diagnostics.
- Preserve HTTP error response bodies for both health checks and generation requests.
- Make GGUF and Rust-native provider health/readiness depend on model-ready health, not merely an open TCP socket.
- Add regression coverage for loading 503 → ready 200 transitions, early runtime exit, health error bodies, generation error bodies, and runtime stderr capture.

## 0.2.1

- Fix Qwen3 live activation: Fast/Balanced requests now explicitly disable thinking while Deep/Maximum enable it.
- Parse llama-server `reasoning_content` as a distinct stream so reasoning traffic is no longer lost or folded into final assistant text.
- Make the live verification probe use Fast processing plus Qwen `/no_think` fallback for deterministic final-content output.
- Preserve provider/runtime failure details in the verification report instead of collapsing them to `LIVE_GATE_FAILED`.
- Keep v0.2.0 conversations, projects, model registry, runtime/model assets, and SQLite schema fully compatible.

## 0.2.0

- Add the first native AetherAI desktop application using the AetherForge/Aether Terminal DragonGlass visual language.
- Add collapsible, resizable, persistent navigation and Processing panels.
- Keep one conversation model: heavy Work is integrated into normal chat rather than a separate Work mode.
- Add real local streamed inference contracts and remove the production mock provider path.
- Add AetherGGUF with managed llama.cpp-compatible local runtime supervision and OpenAI-compatible SSE streaming.
- Add AetherNative through a local mistral.rs-compatible Rust service boundary.
- Add live model import, selection, registry state, pinned starter-model download, resumable download, and SHA-256 verification.
- Pin Qwen3 0.6B Q4_K_M (Apache-2.0) as the starter GGUF model.
- Pin llama.cpp b10649 Linux/Windows x86_64 Vulkan runtime assets with exact SHA-256 values.
- Add explicit Network Off / Ask / On policy; Ask remains the default.
- Add project discovery/recovery, AI-assistance evidence classification, and safe scan exclusions.
- Add project-scoped permissioned process/build/test execution with auditable Activity records.
- Add typed Linux/Windows system-integration foundations for platform paths, telemetry, processes, notifications, and open-path behavior.
- Add SQLite schema v2 migration preserving v0.1.9 conversations and removing legacy `aetherai/mock` selection.
- Add first-run, Models, Projects, Activity, and Processing desktop surfaces.
- Add separate build verification and real-live-model verification endpoints; full v0.2.0 PASS requires actual streamed model output.
- Keep Linux and Windows packaging contracts separate and preserve bundled Rust/offline vendored-source verification.

## 0.1.9

- Fix portable Clippy execution by exposing the real bundled `rustc`/`rustdoc` binaries to Cargo.
- Inject the relocated standalone sysroot through `RUSTFLAGS` and `RUSTDOCFLAGS`.
- Verify bundled Rust formatting, Clippy, tests, release build, CLI smoke, and Linux self-contained gates.

# AetherAI

**AetherAI v0.2.2** is a free, local-first Rust assistant with a native AetherForge/DragonGlass desktop application. Normal chat and heavy Work use the same conversation identity: projects, files, terminal/build/test execution, Activity, model controls, project discovery, and system integration are capabilities of that chat rather than a separate Work product.

## Live inference

Production runtime no longer uses `MockProvider`. AetherAI supports:

- **AetherGGUF** — managed local llama.cpp-compatible inference with streamed OpenAI-compatible SSE responses.
- **AetherNative** — local mistral.rs-compatible Rust inference service path.
- real model registry/import, model selection, streamed responses, and explicit failure when no live model is configured.

The pinned starter path is **Qwen3 0.6B Q4_K_M** (Apache-2.0). The Linux/Windows GGUF runtime is pinned to llama.cpp **b10649** Vulkan assets. Runtime/model downloads require explicit `Network: On`, resume partial downloads, and verify SHA-256 before activation.

## Desktop

`aetherai-desktop` is a native desktop app with:

- AetherForge/Aether Terminal DragonGlass styling;
- collapsible/resizable navigation and Processing panels;
- Fast, Balanced, Deep, Maximum, and Custom processing presets;
- unified chat + heavy Work commands (`/run <command>` inside an attached project);
- project recovery with local AI-assistance provenance evidence;
- live Models page that can install the verified GGUF runtime + starter model;
- Activity audit for project commands;
- typed Linux/Windows system-integration boundaries;
- SQLite v1→v2 migration preserving v0.1.9 conversation identity while removing legacy mock selection.

## Verification

The source/package keeps the bundled Rust toolchain, `Cargo.lock`, and vendored crates. Run:

```bash
bash scripts/verify-linux-self-contained.sh
```

A fully activated release additionally requires a real local model generation smoke. To acquire the pinned live assets and run every gate:

```bash
AETHERAI_INSTALL_LIVE_ASSETS=1 bash scripts/verify-linux-self-contained.sh
```

The verifier does **not** manufacture a live PASS. Without an actual model/runtime it reports the build gates separately and leaves `AETHERAI_V0_2_2_VERIFY=FAIL` until real streamed generation succeeds.


## v0.3.0 Files / RAG / Memory milestone

AetherAI v0.3.0 begins the local Files/RAG/Memory milestone while preserving the verified v0.2.2 live-model path. Core retrieval is local-first; lexical and symbol retrieval do not require embeddings or a paid hosted provider.

# AetherAI — Architecture Design Specification

**Date:** 2026-08-30  
**Status:** Approved  
**Project:** AetherAI  
**Primary implementation language:** Rust  

## 1. Product Definition

AetherAI is a completely free, open-source, local-first AI assistant written primarily in Rust. Its goal is to provide a ChatGPT-class *capability surface*—conversation, files, memory, tools, web access, code execution, multimodal input/output, agents, projects, and artifact creation—without requiring a paid API, subscription, cloud account, or proprietary hosted model.

AetherAI does **not** attempt to redistribute proprietary OpenAI model weights. Instead, it provides a unified assistant runtime capable of using open-weight models locally through both Rust-native inference and compatibility inference backends.

The application must remain useful when completely offline, except for features whose nature requires network access, such as web browsing or downloading models.

## 2. Non-Negotiable Product Constraints

1. **100% free core runtime.** No core feature may require a paid API, usage credits, or subscription.
2. **Open source.** First-party AetherAI source code is published under an OSI-compatible copyleft license; the recommended baseline is AGPL-3.0-or-later unless changed before public release.
3. **Rust-first.** All first-party services, orchestration, state management, tool routing, UI backend, persistence, and pure-native inference components are implemented in Rust.
4. **Local-first.** Conversations, memory, model configuration, project state, files, embeddings, and tool state default to local storage.
5. **One chat system.** There is no separate “Work” product mode. Any normal chat can become a persistent workspace.
6. **Open model weights.** AetherAI supports locally installed model weights whose licenses permit user download/use. Model licenses remain the responsibility of their upstream publishers and are displayed before installation.
7. **No hidden provider lock-in.** All model execution is accessed through a stable internal provider trait.
8. **User-controlled processing.** Reasoning depth, model selection, context budget, sampling, hardware placement, memory, tools, and agent limits remain user-configurable.
9. **Graceful degradation.** Missing GPU acceleration, unavailable web access, or an unsupported tool must not make ordinary chat unusable.
10. **Auditable tool execution.** Tool calls and side effects are permissioned, logged, inspectable, and cancellable where technically possible.

## 3. Unified Chat + Workspace Model

AetherAI treats every conversation as the same underlying object. A chat begins lightweight and gains workspace capabilities only as they are used.

Each conversation may contain:

- ordinary message history;
- system/developer/user role context;
- attachments and generated files;
- a persistent project directory;
- source code and build artifacts;
- local retrieval indexes;
- chat-scoped and project-scoped memory;
- tool execution history;
- terminal sessions;
- web research results and citations;
- generated documents, spreadsheets, images, audio, and other artifacts;
- agent/subagent runs;
- checkpoints and resumable state;
- model and processing settings.

There is no mode transition such as “move to Work.” The same chat simply acquires richer state.

### 3.1 Conversation Types

The UI may label or filter conversations for convenience, but these are metadata only:

- Chat
- Coding
- Research
- Project
- Document
- Analysis
- Agent task

They all use the same storage and runtime model.

## 4. High-Level Architecture

```text
┌──────────────────────────────────────────────────────────────┐
│                         AetherAI UI                           │
│ desktop / local web / terminal client                        │
└──────────────────────────────┬───────────────────────────────┘
                               │
┌──────────────────────────────▼───────────────────────────────┐
│                     AetherAI Application Core                │
│ chats • projects • permissions • settings • artifacts        │
└──────────────────────────────┬───────────────────────────────┘
                               │
┌──────────────────────────────▼───────────────────────────────┐
│                       Agent Runtime                          │
│ planning • context assembly • tools • agents • memory        │
└──────────┬───────────────────┬───────────────────┬───────────┘
           │                   │                   │
┌──────────▼─────────┐ ┌───────▼─────────┐ ┌──────▼──────────┐
│ Model Runtime      │ │ Tool Runtime    │ │ Knowledge       │
│ providers/routing  │ │ sandbox/perm    │ │ RAG/memory      │
└──────────┬─────────┘ └─────────────────┘ └─────────────────┘
           │
   ┌───────┴──────────────────────────────┐
   │                                      │
┌──▼────────────────────┐      ┌──────────▼──────────────────┐
│ Pure Rust Inference   │      │ Compatibility Inference    │
│ Candle/wgpu-class     │      │ llama.cpp/GGUF + adapters  │
│ SafeTensors-native    │      │ optional external runtime  │
└───────────────────────┘      └─────────────────────────────┘
```

## 5. Model Provider Layer

All inference engines implement a common Rust trait. The rest of AetherAI must not depend on engine-specific APIs.

Conceptual interface:

```rust
#[async_trait]
pub trait ModelProvider: Send + Sync {
    async fn capabilities(&self) -> ProviderCapabilities;
    async fn models(&self) -> Result<Vec<ModelDescriptor>>;
    async fn generate(&self, request: GenerationRequest) -> Result<GenerationStream>;
    async fn embed(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse>;
    async fn tokenize(&self, input: &str) -> Result<Tokenization>;
    async fn health(&self) -> ProviderHealth;
}
```

This interface is illustrative; exact API types are set during implementation planning.

### 5.1 Pure Rust Backend

The pure Rust backend is the long-term native engine and must support, incrementally:

- SafeTensors model loading;
- tokenizer loading;
- CPU inference;
- GPU acceleration through Rust-native GPU stacks where practical;
- quantized weights;
- KV caching;
- streaming generation;
- embeddings;
- multimodal models where supported;
- configurable context windows;
- configurable sampling;
- model sharding/offload;
- deterministic seeds where supported.

### 5.2 Compatibility Backend

A compatibility backend provides broad model support before every model architecture is implemented natively.

Initial target:

- GGUF models;
- llama.cpp-compatible local runtimes;
- local OpenAI-compatible HTTP servers;
- additional local inference engines through adapters.

The compatibility backend is optional. AetherAI remains buildable with the pure Rust backend alone.

### 5.3 OpenAI-Compatible Protocol

AetherAI supports the OpenAI-compatible API *protocol* for interoperability with local inference servers and third-party local tools. This does not require or imply use of OpenAI's paid hosted API.

AetherAI should eventually also expose its own local OpenAI-compatible endpoint so existing software can use AetherAI as a local model server.

## 6. Model Manager

A first-party model manager handles model lifecycle.

Required operations:

- discover compatible open-weight models from configured catalogs;
- download with resume support;
- verify checksums;
- show upstream model license before installation;
- import SafeTensors/GGUF from disk;
- remove models;
- inspect metadata;
- benchmark models;
- quantize where supported;
- select CPU/GPU placement;
- estimate RAM/VRAM requirements;
- configure context size and cache;
- define default models by task;
- pin versions/checksums;
- prevent accidental use of incompatible model formats.

Model metadata is stored independently of chat data.

## 7. Processing Controls

AetherAI exposes high-level processing presets and advanced controls.

### 7.1 Presets

- **Fast** — minimize latency and token budget.
- **Balanced** — default general-use setting.
- **Deep** — increase reasoning/token/tool budget.
- **Maximum** — use the configured maximum safe local budget.

Presets are transparent configuration bundles, not hidden modes. Users can inspect and override their values.

### 7.2 Advanced Per-Chat Controls

Where the selected model/runtime supports them:

- model;
- reasoning/token budget;
- context size;
- maximum output tokens;
- temperature;
- top-p;
- top-k;
- repetition/frequency penalties;
- seed;
- CPU threads;
- GPU device;
- GPU/CPU layer placement;
- RAM/VRAM caps;
- KV-cache type/size;
- quantization;
- batch size;
- parallel generation limits;
- agent depth;
- concurrent subagents;
- tool permissions;
- web access;
- code execution;
- computer control;
- memory scope;
- retrieval depth.

Unsupported settings are disabled with an explanation rather than silently ignored.

## 8. Agent Runtime

The agent runtime coordinates models and tools while remaining separate from model inference.

Responsibilities:

- construct context;
- interpret tool-call output from supported models;
- execute tools through the permission layer;
- append results back into the conversation;
- enforce recursion/agent-depth limits;
- support parallel subagents;
- track run state;
- recover from tool/model failures;
- support user cancellation;
- preserve an auditable event log.

The runtime must support both models with native tool calling and models that require prompt-based tool-call adaptation.

## 9. Tool System

Tools implement a typed Rust interface and declare capabilities, permissions, side effects, input schema, and output schema.

Initial tool families:

- filesystem;
- terminal/process execution;
- code build/test;
- web search;
- webpage fetch;
- browser automation;
- calculator;
- local database/query;
- document creation;
- spreadsheet creation;
- image processing/generation adapters;
- audio/STT/TTS adapters;
- Git;
- local applications;
- user-created plugins.

### 9.1 Permissions

Permission scopes include:

- read files;
- write files;
- execute processes;
- network access;
- browser/computer control;
- camera/microphone;
- external device access;
- destructive actions.

Policies may be configured globally, per project, per chat, or per invocation.

## 10. Sandbox and Code Execution

AetherAI provides code execution without giving unrestricted machine access by default.

Execution modes:

1. **Restricted sandbox** — default for generated code.
2. **Project workspace** — access limited to the active workspace.
3. **Host-approved execution** — explicit user permission for broader access.

The sandbox abstraction should support Linux namespaces/containers where available and degrade safely on unsupported systems.

## 11. Memory

Memory is explicit and scoped.

Types:

- message history;
- short-term working memory;
- chat memory;
- project memory;
- user-approved long-term preferences;
- retrieval memory derived from files;
- tool/run checkpoints.

Users can inspect, edit, disable, export, and delete persistent memory.

No data is sent off-device solely for memory functionality.

## 12. Files, RAG, and Knowledge

AetherAI supports local files as first-class chat/workspace objects.

Pipeline:

```text
file
→ type detection
→ parser/extractor
→ structured chunks
→ embeddings
→ local vector/index store
→ retrieval
→ cited context
→ model
```

Requirements:

- incremental indexing;
- content hashes;
- local embedding models;
- hybrid lexical/vector search;
- page/line/section provenance where available;
- citations in answers;
- user-configurable index scope;
- no mandatory remote embedding service.

## 13. Web and Research

Web access is optional and disabled offline.

Capabilities:

- search engine adapters;
- page retrieval;
- HTML/readability extraction;
- browser automation for dynamic pages;
- source provenance;
- citations;
- download handling;
- configurable domain/network policies.

Search APIs that require payment are not dependencies. Free/public adapters and user-provided endpoints may be added as optional plugins.

## 14. Multimodal

AetherAI's architecture supports:

- image input/vision;
- image generation through local models;
- OCR where appropriate;
- speech-to-text;
- text-to-speech;
- audio analysis;
- future video understanding/generation adapters.

All multimodal services use provider interfaces so fully local implementations can be swapped without changing the chat system.

## 15. Artifacts

A chat can generate persistent artifacts directly into its workspace:

- source trees;
- text/Markdown;
- PDFs;
- office documents;
- spreadsheets;
- presentations;
- images;
- audio;
- archives;
- build packages.

Artifacts store provenance linking them to the chat turn/tool run that created them.

## 16. User Interface

The primary desktop experience is a unified ChatGPT-style conversation UI with workspace capabilities embedded into normal chats.

Core surfaces:

- chat sidebar;
- conversation view;
- composer/attachments;
- model selector;
- processing preset selector;
- expandable advanced generation controls;
- file/workspace drawer;
- tool activity drawer;
- model manager;
- memory manager;
- settings;
- local runtime health/telemetry.

No separate “Work” navigation destination is required.

The UI should support keyboard-first operation and remain usable without GPU rendering acceleration.

## 17. Persistence

Recommended baseline:

- SQLite for relational metadata and chat state;
- content-addressed filesystem storage for attachments/artifacts/models;
- SQLite FTS for lexical retrieval;
- pluggable local vector index;
- append-only event records for agent/tool runs where useful.

Database migrations are versioned and reversible where practical.

## 18. Privacy and Security

Defaults:

- local storage;
- no analytics/telemetry unless explicitly enabled;
- no paid cloud dependency;
- secrets stored using OS secret storage where available;
- tool permissions least-privilege by default;
- model downloads checksum-verified;
- plugin manifests signed or explicitly trusted;
- workspace boundaries enforced;
- destructive commands require elevated permission policy.

## 19. Extensibility

Plugin interfaces are versioned and capability-based.

Long-term plugin types:

- model provider;
- tool;
- parser;
- embedding provider;
- speech provider;
- image provider;
- search provider;
- UI extension;
- device integration.

Out-of-process plugins are preferred for strong isolation where feasible.

## 20. Proposed Rust Workspace Boundaries

```text
AetherAI/
├── apps/
│   ├── aetherai-desktop/
│   ├── aetherai-cli/
│   └── aetherai-server/
├── crates/
│   ├── aether-core/
│   ├── aether-chat/
│   ├── aether-agent/
│   ├── aether-model-api/
│   ├── aether-infer-native/
│   ├── aether-infer-compat/
│   ├── aether-model-manager/
│   ├── aether-tools/
│   ├── aether-sandbox/
│   ├── aether-memory/
│   ├── aether-rag/
│   ├── aether-files/
│   ├── aether-web/
│   ├── aether-multimodal/
│   ├── aether-artifacts/
│   ├── aether-storage/
│   ├── aether-permissions/
│   ├── aether-protocol/
│   └── aether-plugin-api/
├── docs/
├── tests/
└── tools/
```

These boundaries are logical targets. The initial implementation should create only crates needed by the first milestone rather than generating empty crates for every future component.

## 21. Error Handling

AetherAI uses typed errors internally and converts them to actionable user-visible failures at subsystem boundaries.

Rules:

- model failure must not corrupt conversation state;
- tool failure is recorded as a tool result/event;
- failed artifact generation does not replace a previous valid artifact;
- database writes use transactions;
- downloads use temporary/incomplete state until checksum verification passes;
- incompatible model configuration fails before generation starts;
- cancellations preserve a consistent partial transcript/event state.

## 22. Testing Strategy

### Unit Tests

- provider capability negotiation;
- sampling/config validation;
- context assembly;
- permission evaluation;
- memory scoping;
- retrieval ranking;
- storage migrations;
- tool schema validation.

### Integration Tests

- chat → model → stream;
- chat → tool → model;
- file → index → cited retrieval;
- workspace code execution;
- model download/import;
- provider fallback;
- cancellation/recovery;
- persistence/reload.

### Contract Tests

Each provider/tool/plugin must pass shared behavioral contracts.

### Host Verification

Release verification should test:

- Rust formatting/lints;
- workspace tests;
- release build;
- database migration roundtrip;
- minimal CPU inference smoke test where model fixture licensing/size allows;
- generated verification report.

## 23. Release Philosophy

AetherAI should use clean semantic versions with one canonical source artifact per version. Every release should include:

- versioned source archive;
- SHA-256 checksums;
- concise changelog/README;
- verification script;
- generated verification `.txt` output suitable for uploading back into a development chat;
- one copy/paste build/verify command targeted at `~/Downloads` for Linux development workflows.

## 24. Milestone Decomposition

The full capability target is too large for one implementation cycle. It is decomposed as follows.

### M0 — Foundation

- Cargo workspace;
- core domain types;
- unified chat/workspace storage;
- model provider trait;
- mock provider;
- CLI chat loop;
- SQLite persistence;
- configuration;
- verification harness.

### M1 — Free Local Text Inference

- compatibility GGUF runtime;
- model manager import;
- streaming text generation;
- processing presets;
- advanced generation controls;
- CPU-first verification.

### M2 — Pure Rust Inference

- SafeTensors-native model loading;
- tokenizer integration;
- initial supported architecture;
- CPU and available GPU acceleration;
- quantization/cache work.

### M3 — Desktop Unified Chat/Workspace UI

- conversation sidebar;
- message view/composer;
- model/runtime controls;
- workspace/file pane;
- normal chats and project chats unified.

### M4 — Tools + Sandboxed Coding

- typed tool protocol;
- filesystem;
- terminal;
- project sandbox;
- build/test tools;
- permissions UI.

### M5 — Files, RAG, Memory

- parsers;
- local embeddings;
- indexing;
- citations;
- persistent memory controls.

### M6 — Web Research

- free search adapters;
- webpage retrieval;
- browser adapter;
- source citation pipeline.

### M7 — Agents

- tool loops;
- subagents;
- configurable processing depth;
- parallelism limits;
- checkpoints/recovery.

### M8 — Multimodal + Artifacts

- vision;
- image generation adapter;
- STT/TTS;
- richer artifact generation.

### M9 — Local API + Ecosystem

- OpenAI-compatible local API endpoint;
- plugin SDK;
- provider/tool extensions;
- external client interoperability.

## 25. Explicit Non-Goals for Initial Releases

To prevent the project from collapsing under its final scope, early releases will not attempt to:

- train a frontier foundation model from scratch;
- clone proprietary OpenAI model weights;
- require a hosted backend;
- support every model architecture on day one;
- ship every multimodal modality before text/chat/tool foundations are stable;
- expose unrestricted host control by default.

## 26. Definition of Success

AetherAI reaches its intended product state when a user can install it on a supported machine, download/import a legally available open model, and use a single unified chat interface for ordinary conversation, research, files, coding, local memory, agents, artifacts, multimodal work, and persistent projects without paying for an API or subscription.

The user may choose exactly how much compute and reasoning to spend, which model weights to run, what hardware resources to allocate, and what tools/memory are permitted.


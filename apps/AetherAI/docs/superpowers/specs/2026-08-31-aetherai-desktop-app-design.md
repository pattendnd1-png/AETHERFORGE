# AetherAI v0.2.0 — Desktop App Design Specification

**Date:** 2026-08-31  
**Status:** Approved architecture; live-runtime activation amendment incorporated; pending implementation planning  
**Baseline:** AetherAI v0.1.9  
**Primary language:** Rust  
**Primary desktop targets:** Linux x86_64 and Windows x86_64, as separate canonical platform artifacts

## 1. Purpose

AetherAI v0.2.0 turns the verified v0.1.9 Rust core into the first native desktop application while preserving AetherAI's local-first, free, open-source architecture.

The application must present a single unified conversation system. Ordinary chat, coding, project work, research, files, terminal execution, agents, artifacts, multimodal input, and system integration are capabilities of the same conversation object. AetherAI must never split the product into separate “Chat” and “Work” products or incompatible conversation types.

v0.2.0 is the desktop foundation milestone. It establishes the durable GUI, persistence, project/workspace, permissions, discovery, processing controls, activity model, and system-integration boundaries that later v0.2.x releases can deepen with additional inference, browser, vision, voice, image, and automation engines without redesigning the application shell.

## 2. Baseline Compatibility

The current v0.1.9 source already provides:

- `aether-core`: conversations, roles, messages, workspace state, processing presets, generation settings;
- `aether-model-api`: provider-neutral model boundary;
- `aether-chat`: chat orchestration over `ModelProvider`;
- `aether-storage`: SQLite conversation persistence;
- `aetherai-cli`: command-line access to the same core;
- `xtask` and portable verification: formatting, Clippy, tests, release build, CLI smoke, bundled Rust/offline verification.

v0.2.0 must extend these boundaries rather than fork desktop-only copies of chat, settings, storage, or model behavior.

## 3. Non-Negotiable Product Rules

1. **One conversation system.** Heavy Work capabilities are integrated into normal chat.
2. **Local-first.** Core chat, project state, memory, files, build/test workflows, and installed-model inference do not require a cloud account.
3. **No paid provider requirement.** Paid OpenAI or other hosted APIs are not required for core operation.
4. **Rust-first.** First-party application logic, services, permissions, persistence, project discovery, process execution, and system integration are Rust.
5. **Separate platform artifacts.** Linux and Windows releases are packaged separately.
6. **Canonical versioning.** Every changed release receives a new semantic version; no RC/rN correction suffixes.
7. **Auditable side effects.** File writes, process execution, network access, computer control, and destructive operations are visible and permissioned.
8. **Recoverable work.** Conversation, workspace, generated artifacts, task logs, and checkpoints survive restart.
9. **Capability-gated system power.** OS integration is explicit, inspectable, and scoped.
10. **AetherForge visual identity.** AetherAI must look like a native AetherForge application, with Aether Terminal as the closest visual reference.


## 3A. Live Runtime Activation — No Mock Production Mode

AetherAI v0.2.0 is not permitted to ship as a GUI over `MockProvider`, a simulated assistant, or an offline-only demonstration runtime. `MockProvider` remains available only to automated tests and explicit developer test builds. Normal desktop and CLI startup must use a real production provider registry.

The live runtime contract is:

- real local model discovery/import/download;
- real model loading;
- real tokenization and context accounting;
- real token-streamed generation;
- real provider health/error reporting;
- real project-scoped file/process/Git operations when permissioned;
- real persistence and recovery;
- real system-integration calls for every control advertised as available;
- no fabricated success responses when a provider or capability is unavailable.

If no usable model is installed, AetherAI remains a functional desktop application but chat generation reports **No live model configured** and guides the user to Models/First Run. It must never silently fall back to mock assistant text.

### 3A.1 Runtime provider stack

The first production stack is dual-path:

```text
ModelProvider
   ├── AetherGGUF
   │     └── managed llama.cpp/gguf-server-family runtime
   │         CPU / CUDA / HIP-AMD / Vulkan as supported by platform build
   │
   └── AetherNative
         └── Rust-native mistral.rs/Candle-class runtime
             SafeTensors/native model path + supported GGUF/quantized formats
```

`aether-model-api` remains the application-facing contract. Backend-specific APIs never leak into chat, storage, workspace, or GUI crates. The live runtime is hosted behind `aetherai-model-service` so model crashes do not take down the desktop application.

### 3A.2 Network is capability-gated, not globally disabled

AetherAI remains capable of completely local/offline operation after models and required assets are present, but the application is not globally forced into offline mode. Network-capable features use an explicit policy:

- **Off** — no external network requests;
- **Ask** — request permission before external access;
- **On** — allow approved network capability for the current scope.

Default policy is **Ask**. Model downloads, metadata/license lookup, web/research, and other network features must use the same permission/audit system. No paid provider or cloud account is required for core operation.

### 3A.3 No fake capability controls

A UI action may be shown as enabled only when a real implementation is available and healthy. Unsupported future capabilities are hidden or disabled with an explicit explanation. v0.2.0 must not simulate inference, tool execution, browsing, downloads, system telemetry, or file changes.

## 4. Desktop Technology Direction

The first desktop client is a native Rust application under:

```text
apps/aetherai-desktop/
```

The preferred UI stack is `egui`/`eframe` with GPU-backed rendering where available, because it keeps the application Rust-native and portable across Linux and Windows. Desktop-specific visual behavior must be abstracted sufficiently that the application can gracefully fall back when compositor transparency, blur, or platform-specific decorations are unavailable.

The GUI must not own model inference, indexing, long-running process execution, or broad OS integration directly. Those responsibilities sit behind service boundaries so the UI can remain responsive and recover from service failures.

## 5. Permanent Visual Language — AetherForge / Aether Terminal DragonGlass

AetherAI's theme is not generic ChatGPT, stock egui, or a conventional gray developer tool. It uses the AetherForge DragonGlass application language, with Aether Terminal as the primary visual reference.

### 5.1 Core visual rules

- very dark smoked-glass application surfaces;
- strong translucency where the OS/compositor supports it;
- an opaque dark fallback that preserves readability when transparency is unavailable;
- violet, indigo, and deep-blue active accents;
- pale near-white / light-lavender text and glyphs;
- compact title bars and dense professional spacing;
- fully rounded compact window controls;
- rounded floating panels and readout cards;
- minimal borders and no bulky icon backplates;
- subtle highlight/glow only for active or selected controls;
- translucent menus, tooltips, popovers, and context menus;
- AetherForge fonts and iconography when present on the host;
- clean fallback fonts/icons on generic Linux/Windows installations.

### 5.2 Aether Terminal relationship

Code blocks, terminal output, build logs, test logs, tool activity, verification output, and command history use an Aether Terminal-inspired surface:

- dark terminal field;
- compact monospace typography;
- brighter indigo identity/accent text;
- restrained syntax/status differentiation;
- no oversized web-style padding;
- readable long-line wrapping and horizontal scrolling where appropriate.

### 5.3 Panel composition

The desktop shell has three independently useful regions:

```text
┌──────────────┬────────────────────────────┬──────────────────┐
│ Navigation   │ Conversation               │ Processing       │
│ / Projects   │ / Work activity            │ / Capabilities   │
└──────────────┴────────────────────────────┴──────────────────┘
```

Both left navigation and right processing panels are collapsible and resizable. The center conversation expands into the freed space.

### 5.4 Responsive behavior

- panel open/closed state persists across restarts;
- panel widths persist across restarts;
- narrow windows may auto-collapse the right panel first;
- users can override automatic collapse;
- the center composer remains usable at minimum supported width;
- UI scale/zoom persists per user.

## 6. Main Navigation

The collapsible left rail contains:

```text
+ New Chat

Chats
Projects
Files
Models
Artifacts

Search
Activity
Settings
```

There is no Work navigation destination. Work is capability state inside a normal conversation.

### 6.1 Conversation operations

Chats support:

- create;
- open;
- rename;
- pin;
- archive;
- delete;
- duplicate;
- export;
- move/associate with a project;
- search by title/content/project metadata.

## 7. Unified Chat + Heavy Work Model

A normal conversation may begin with only message history and model settings. It can acquire any of the following without changing identity:

- one or more workspace roots;
- project files;
- source-code context;
- Git repository state;
- terminal/process execution;
- build/test/lint/format workflows;
- local RAG/indexing;
- project memory;
- web/research;
- artifacts and documents;
- subagents;
- multimodal input/output;
- system/process/display/audio integration;
- checkpoints and resumable work state.

The database identity of the conversation remains the same throughout these transitions.

### 7.1 Workspace modes

A conversation can have:

- no workspace;
- a single project root;
- multiple project roots;
- a temporary scratch workspace;
- a recovered/discovered existing project.

Attaching a workspace records a durable relationship to an existing filesystem path. Source trees are not duplicated unless explicitly requested.

### 7.2 Work permission presets

At minimum:

- **Read Only** — inspect/search/summarize approved project roots;
- **Project Write** — modify approved roots;
- **Project + Build** — modify plus execute approved development/build/test commands;
- **Full Development** — project access plus terminal, Git, subagents, and approved external tools.

These presets expand into inspectable individual permissions.

## 8. Main Conversation Surface

The central surface supports:

- user/assistant/tool messages;
- Markdown;
- syntax-highlighted code;
- copy actions;
- inline file attachments;
- citations/source references;
- generated artifacts;
- image/audio attachments when supported;
- collapsible tool activity;
- build/test/verification summaries;
- agent/subagent activity;
- streaming model output;
- stop/cancel controls;
- regenerate/retry actions.

### 8.1 Composer

The composer is fixed to the bottom of the conversation and supports:

- multiline text;
- `Enter` to send;
- `Shift+Enter` for newline;
- drag/drop files and folders;
- paste image/file content where supported;
- attachment picker;
- screenshot/voice controls when those capabilities are enabled;
- stop-generation while a run is active.

## 9. Processing Panel

The right panel is collapsible, resizable, persistent, and independently toggleable from the top bar and keyboard shortcut.

It exposes simple presets first:

- Fast;
- Balanced;
- Deep;
- Maximum;
- Custom.

Presets are transparent bundles. The user can inspect and override the underlying values.

### 9.1 Advanced processing controls

Where supported by the selected backend/model:

- model;
- backend;
- context size;
- max output;
- temperature;
- top-p;
- top-k;
- seed;
- reasoning budget;
- tool-loop budget;
- agent depth;
- concurrent subagents;
- CPU threads;
- GPU selection/layer placement;
- RAM/VRAM caps;
- KV cache settings;
- memory;
- files/RAG;
- web/research;
- code execution;
- computer control.

Unsupported controls are disabled with an explanation rather than silently ignored.

### 9.2 Settings inheritance

```text
Global defaults
      ↓
Project defaults
      ↓
Conversation overrides
```

A conversation override never silently mutates global defaults.

## 10. Project Discovery and Recovery

AetherAI can scan approved local locations for existing development/work projects, including projects that appear to have been created or substantially developed using ChatGPT/OpenAI-assisted workflows.

### 10.1 Discovery roots

Initial configurable candidates include:

- Downloads;
- Documents;
- Desktop;
- Projects/Source/repos-style directories;
- user-selected directories;
- mounted development drives.

Whole-system scanning is explicit, not automatic.

### 10.2 Project detection

Detect common project structures such as:

- Git repositories;
- Rust/Cargo workspaces;
- Node/package.json projects;
- Python projects;
- CMake/Meson/Ninja projects;
- game/source projects;
- shell/script tool projects;
- arbitrary source trees with recognizable project metadata.

### 10.3 AI-origin evidence

AetherAI may classify AI-assisted provenance using local evidence such as:

- ChatGPT/OpenAI references;
- verification files and release artifacts;
- conversation-export metadata;
- AI-assisted build instructions;
- README/change-log language;
- project-specific assistant notes;
- known AetherAI/AetherForge verification conventions.

Classification is explicitly probabilistic:

- **Confirmed**;
- **Likely**;
- **Uncertain**;
- **Normal Project**.

AetherAI must not claim confirmed AI origin without sufficient evidence.

### 10.4 Scan exclusions

High-noise and sensitive paths are excluded by default, including:

- `target/`;
- `node_modules/`;
- `.git/objects/`;
- virtual environments;
- cache directories;
- browser caches;
- password stores;
- SSH/GPG key stores.

### 10.5 Import behavior

Importing a discovered project registers metadata and workspace relationships. It does not copy the project tree by default.

First-run UI presents discovered projects with selection controls; nothing is imported automatically.

## 11. Files, RAG, Memory, and Project Knowledge

AetherAI indexes attached files and workspace content locally.

Retrieval modes include:

- keyword/full-text search;
- semantic/vector search;
- symbol-aware code search;
- hybrid retrieval;
- recent-file weighting;
- project-memory weighting;
- Git-history retrieval;
- conversation-history retrieval.

For source code, indexing should evolve toward functions, structs, modules, manifests, imports, tests, and symbol references rather than relying only on plain-text chunks.

### 11.1 Memory layers

- immediate conversation context;
- working context;
- project memory;
- retrieved project knowledge;
- long-term summaries;
- artifact relationships;
- checkpoints.

Persistent memory is inspectable, editable, exportable, and deletable.

## 12. Local Model Manager and Inference Contracts

The Models page contains:

- Installed;
- Available;
- Import Model;
- Downloads;
- Benchmarks.

It supports two **live production** provider families behind `aether-model-api`:

```text
ModelProvider
   ├── AetherNative   (Rust-native / SafeTensors-class path)
   └── AetherGGUF     (GGUF / llama.cpp-compatible path)
```

The rest of the application must not depend on backend-specific APIs. `MockProvider` is test-only and may not be selected by normal desktop/CLI runtime configuration.

### 12.1 Model metadata

Display:

- name;
- architecture;
- parameter count;
- format;
- quantization;
- file size;
- context limit;
- RAM/VRAM estimate;
- CPU/GPU compatibility;
- backend;
- benchmark results;
- upstream license;
- last-used information.

### 12.2 Model discovery/download

AetherAI can scan approved model directories for GGUF/SafeTensors assets and register them in place. Downloads are **live network operations** mediated by the permission layer, resumable, integrity checked, and must show upstream license information before installation. Imported or downloaded models must be loadable by a production provider before being reported as Ready.

### 12.3 Hardware fitting

AetherAI evaluates CPU, RAM, GPU, VRAM, available memory, and current load to recommend safe model/context/offload settings. Recommendations are user-overridable.

### 12.4 Auto routing

`Model: Auto` can route normal chat, coding, reasoning, vision, embedding, summarization, and utility work to appropriate installed local models. A conversation can lock to one model.

## 13. Tool, Terminal, Git, Build, and Agent Runtime

Heavy Work operations are represented as auditable tool activity in the same conversation.

### 13.1 Process runner

The Rust process runner supports project-scoped execution of development tools such as:

- Cargo/Rust tools;
- Git;
- CMake/Meson/Ninja;
- Python;
- Node/npm;
- shell commands;
- test runners;
- formatters/linters;
- packagers;
- project executables.

Every process records command, working directory, stdout/stderr, start/end state, exit code, associated conversation/project, and cancellation state.

### 13.2 Agent workers

A normal conversation may spawn specialist workers such as coding, testing, research, documentation, and review agents. Their findings return to the parent conversation and remain inspectable.

## 14. Browser, Research, Artifacts, and Documents

Web access is a capability with policy `Off`, `Ask`, or `On` per applicable scope. Network access is mediated by a dedicated service boundary rather than arbitrary model access to sockets.

Research can use search, page fetch, documentation lookup, source comparison, and citation capture.

Generated artifacts can include source archives, patches, verification logs, text/Markdown, PDF, DOCX, XLSX/CSV, JSON/YAML, images, diagrams, reports, presentations, and platform packages as supported by installed capability providers.

Artifacts retain relationships to originating conversation, project, source inputs, creation time, version, and revisions.

## 15. Multimodal Contracts

The unified conversation may gain:

- vision/image understanding;
- screenshot/window/region capture;
- local image generation/editing adapters;
- speech-to-text;
- text-to-speech;
- voice conversation;
- optional guided or interactive computer control.

Sensor/control indicators are always visible while active:

- microphone;
- screen;
- camera;
- computer control.

Sensor access never starts invisibly.

## 16. System Integration Layer

A first-class Rust subsystem provides stable OS-facing interfaces:

```text
crates/aether-system/
```

Higher layers use typed abstractions rather than scattering platform-specific code throughout the GUI.

### 16.1 Responsibilities

- notifications;
- clipboard;
- system tray;
- startup registration;
- file associations;
- `aetherai://` protocol/deep links;
- open-with/context actions;
- process discovery;
- process launching/tracking;
- hardware telemetry;
- power/session state;
- network state;
- display information;
- audio-device discovery;
- camera discovery;
- file watching;
- resource-budget reporting;
- platform permission integration.

### 16.2 Linux implementation

The Linux implementation may integrate with:

- Wayland;
- XDG Desktop Portal;
- D-Bus;
- `systemd --user`;
- PipeWire;
- desktop notifications;
- XDG MIME/open integration;
- file watchers;
- platform CPU/GPU telemetry.

Background first-party components run as user services where appropriate. Normal operation must not require a root daemon.

### 16.3 Windows implementation

The Windows implementation may integrate with:

- Win32/Windows Runtime APIs;
- Windows notifications;
- system tray;
- Startup registration;
- file associations;
- URI protocol handler;
- clipboard;
- process APIs;
- audio endpoints;
- camera access;
- known folders;
- file watchers;
- power/session events;
- system/GPU telemetry.

Normal operation must not require Administrator rights.

### 16.4 Deep links

AetherAI owns an optional local URI scheme:

```text
aetherai://chat/<id>
aetherai://project/<id>
aetherai://artifact/<id>
aetherai://model/<id>
```

Notifications and integrations can return the user to the exact local resource.

### 16.5 Tray and notifications

Tray actions include Open AetherAI, New Chat, Voice Conversation, Current Tasks, Pause Background Work, Unload Models, Settings, and Quit.

Notifications can deep-link back to the relevant conversation/project/task.

### 16.6 Process awareness

AetherAI distinguishes processes it launched from unrelated host processes. Processes it launches have richer tracking and can be associated with a project/conversation.

## 17. AetherForge Integration

On AetherForge hosts, AetherAI integrates with existing first-party system ownership rather than duplicating it.

### 17.1 Resource governor

AetherAI publishes inference/agent CPU, GPU, RAM, VRAM, foreground/background, and latency-priority information and can consume system resource budgets. It must not create a competing whole-system governor.

### 17.2 Audio ownership

Where AetherForge's authoritative audio service/AetherStream is present, AetherAI consumes its selected microphone/output and routing interfaces rather than independently taking ownership of the PipeWire graph.

### 17.3 Display awareness

AetherAI treats exposed desktop displays as distinct capture/control surfaces and supports multi-monitor geometry, scale, refresh information, and capture-target selection.

Generic Linux/Windows hosts use normal platform fallbacks.

## 18. Permission Center

Settings contains a dedicated permission center. Permissions are scoped by operation and, where applicable, by chat/project/global context.

Categories include:

- files/workspace;
- terminal/process execution;
- Git/destructive repository operations;
- network/web/model download;
- microphone;
- screen capture;
- camera;
- computer control;
- clipboard;
- background agents/tasks;
- startup/system integration.

Applicable states include:

- Blocked;
- Ask Every Time;
- Allow This Chat;
- Allow This Project;
- Always Allow.

Destructive operations require stricter treatment than routine approved project edits.

## 19. Activity Center and Background Tasks

Activity gives a system-wide view without creating a separate Work mode.

It shows active/recent:

- model inference;
- downloads;
- agents;
- builds/tests;
- research;
- tool calls;
- file changes;
- terminal commands;
- network requests;
- automation/task runs.

Long-running tasks may continue while the main window is minimized when the user has allowed background execution. They remain visible and cancellable in Activity.

## 20. Persistence and Recovery

Persist at minimum:

- window position/size;
- UI scale;
- left panel open/closed and width;
- right panel open/closed and width;
- last conversation;
- project/workspace relationships;
- processing/model settings;
- permission overrides;
- generated artifact metadata;
- last build/test state;
- tool logs;
- checkpoints;
- recoverable task state.

If the GUI or a worker service crashes, stored conversations must remain safe. Service failure must be surfaced without inserting fake assistant turns.

## 21. Service Boundaries

The target service fabric is conceptually:

```text
AetherAI Desktop
     │
     ├── Application Core
     │
     ├── Model Service
     ├── Workspace/Indexer Service
     ├── Tool/Process Service
     ├── Browser/Research Service
     ├── Media Service
     └── System Integration Service
```

Not every service must become an independent OS process in the first implementation. Boundaries are logical first and may be promoted to process/IPC isolation where crash containment, permissions, or long-running execution justify it.

## 22. Storage Evolution

`aether-storage` evolves from schema v1 while preserving existing v0.1.9 conversations.

New storage domains will include, incrementally:

- UI/session state;
- project registry;
- project-discovery evidence;
- workspace roots;
- permission policies;
- artifacts;
- model registry;
- task/activity records;
- indexes/memory metadata;
- checkpoints.

Schema migrations must be forward-safe and tested against a v0.1.9 database fixture.

## 23. First-Run Experience

First launch presents:

1. system check;
2. existing-project discovery;
3. existing-model discovery;
4. model-storage selection;
5. hardware optimization recommendation;
6. permission defaults;
7. ready screen.

Nothing destructive, externally networked, or system-wide is silently enabled during first run.

## 24. Settings Structure

Settings sections:

- General;
- Appearance;
- Chats;
- Models;
- Processing;
- Projects;
- Memory;
- Files & RAG;
- Tools & Terminal;
- Web & Research;
- Agents;
- Voice;
- Vision & Images;
- Computer Control;
- System Integration;
- Permissions;
- Storage;
- Privacy;
- Advanced;
- About.

## 25. v0.2.0 Delivery Scope

v0.2.0 must deliver the first functional desktop shell and durable interfaces, specifically:

1. `apps/aetherai-desktop` native Rust application;
2. DragonGlass/Aether Terminal-aligned theme foundation;
3. collapsible/resizable left navigation and right Processing panel;
4. conversation list, create/open/rename/archive/delete basics;
5. rich central conversation/composer over the existing chat core;
6. SQLite migration preserving v0.1.9 data;
7. persistent UI/session state;
8. workspace attachment and project metadata;
9. initial local project discovery/recovery UI and scanner;
10. initial file/project permission model;
11. initial project-scoped process runner and auditable Activity records;
12. processing preset editor bound to existing generation settings;
13. model registry UI bound to `aether-model-api` capabilities;
14. real production local inference with `AetherGGUF` and at least one working `AetherNative` path;
15. live model import/download/load/generation flow with streaming output;
16. `MockProvider` restricted to tests/developer-only builds;
17. system integration abstraction with Linux/Windows implementations for the subset needed by v0.2.0;
18. first-run flow;
19. crash-safe error presentation;
20. Linux and Windows packaging contracts kept separate;
21. self-contained verification updated to include desktop-specific tests/build checks and live-provider gates.

Broad model-architecture coverage, advanced browser automation, production image generation, voice, and autonomous computer-control engines may continue in subsequent v0.2.x releases, but **real text inference and real local model management are no longer deferrable**. v0.2.0 must answer through a live local provider and must not depend on `MockProvider` outside tests.

## 26. Testing and Verification Gates

The existing portable gates remain mandatory and gain desktop coverage.

Minimum gates:

- `cargo fmt --check`;
- `cargo clippy --workspace --all-targets -- -D warnings`;
- workspace unit/integration tests;
- release build;
- CLI smoke test;
- desktop core-state tests;
- storage migration test from v0.1.9 schema;
- panel persistence tests;
- unified chat/workspace identity test;
- permission-scope tests;
- project-discovery classification tests;
- process-runner audit tests;
- platform abstraction contract tests;
- desktop launch/smoke test where the host environment permits;
- production provider selection test proving normal runtime cannot select `MockProvider`;
- live local model-service health/load/generation smoke test using the packaged integration fixture or approved local test model;
- token-streaming integration test;
- model import/registry/load-state test;
- network-policy test proving Off/Ask/On behavior;
- offline/self-contained Linux verification;
- Windows builder/package contract verification.

Every packaged release generates a version-specific verification text file suitable for uploading back into ChatGPT/AetherAI to continue development.

## 27. Acceptance Criteria

v0.2.0 is acceptable when:

1. the desktop app launches as a real native application;
2. it visually reads as part of the AetherForge/Aether Terminal family;
3. the left rail and Processing panel collapse, resize, and persist correctly;
4. a normal conversation can acquire a workspace without changing conversation identity;
5. chats persist through restart using migrated storage;
6. an existing project can be discovered, classified, registered, and opened into a normal conversation;
7. project-scoped process/tool activity is permissioned and auditable;
8. processing settings shown in the GUI actually update the shared core state;
9. the application can represent model/provider capability and health without desktop-specific provider logic;
10. Linux/Windows system integration is accessed through typed abstractions;
11. a worker/service failure does not corrupt the conversation database or create fake assistant content;
12. the standard verification suite passes;
13. the release produces separate canonical Linux and Windows platform artifacts/contract outputs;
14. the generated verification `.txt` reports the final PASS/FAIL state concisely;
15. normal desktop/CLI runtime cannot produce `AetherAI mock:` or select `aetherai/mock`;
16. at least one real local model can be imported or downloaded, loaded, and used for streamed assistant generation;
17. network-capable features are available when permissioned rather than being globally disabled;
18. loss/crash of the live model service reports a real error without fake assistant content or conversation corruption.

## 28. Out of Scope for the First v0.2.0 Implementation Pass

The following are architectural commitments but are not required to be fully engine-complete in the first desktop implementation pass:

- broad production-grade Rust-native transformer architecture coverage beyond the models validated for v0.2.0;
- complete coverage of every llama.cpp/GGUF model architecture beyond the validated v0.2.0 compatibility set;
- full browser automation;
- full autonomous desktop mouse/keyboard control;
- production image generation/editing engine;
- production local STT/TTS voice stack;
- all artifact formats;
- remote/LAN inference workers.

Interfaces and UI affordances may be present only when they accurately report capability availability. AetherAI must not present a nonfunctional control as working. This out-of-scope list does **not** permit mock text generation: production chat inference is mandatory in v0.2.0.

## 29. Design Principle Summary

AetherAI is one local-first assistant application with one conversation model.

```text
                 NORMAL AETHERAI CHAT
                         │
       ┌─────────────────┼─────────────────┐
       │                 │                 │
    Intelligence      Workspace          System
       │                 │                 │
   local models       files/Git       processes
   reasoning          terminal         hardware
   vision             builds           displays
   voice              agents           audio
   research           artifacts        notifications
   memory             documents        clipboard
                                         OS control
```

“Work” is the heavy capability layer of that normal chat, not another product surface. The desktop app must feel visually native to AetherForge, while its internal service boundaries keep models, tools, projects, and operating-system integration modular and auditable.

# Darkstone-RS — Clean-Room Rust Reimplementation + Reforged Renderer

Date: 2026-08-25
Status: Approved architecture; implementation staged
Project codename: Darkstone-RS

## 1. Purpose

Darkstone-RS is a from-scratch Rust reimplementation of the 1999 action-RPG Darkstone. The project has two equally important goals:

1. Reproduce the original game's gameplay, world behavior, save semantics, asset interpretation, UI flow, quests, combat, AI, procedural content, and co-op behavior closely enough to run against a user's legally obtained original game data.
2. Provide a built-in modern 3D "Reforged" renderer with materially more realistic lighting, materials, effects, animation, terrain, and presentation while preserving Darkstone's gothic-fantasy identity.
3. Extend the completed game into authoritative online play, then two-hero-per-player team play, and finally a persistent open-world MMORPG without forking the core simulation.

The project is not a binary translation or decompilation of the original executable. Original copyrighted data is not bundled. The engine consumes user-owned game data or future replacement/free assets.

## 2. Non-goals

- Do not redistribute original Darkstone textures, audio, music, models, cinematics, maps, text, or other copyrighted assets.
- Do not require Wine/Proton for the reimplemented game runtime.
- Do not make experimental ray tracing a baseline requirement.
- Do not rewrite gameplay rules merely to accommodate modern graphics.
- Do not build separate competing engines for Classic and Reforged modes.
- Do not require high-end hardware to play the compatibility path.
- Do not begin MMO persistence, zone sharding, or large-scale social systems before the offline campaign compatibility gate passes.
- Do not make clients authoritative for combat, loot, inventory, currency, companion AI, or quest progression.
- Do not use peer-to-peer hosting as the canonical MMO architecture.

## 3. Canonical architecture

One canonical simulation supports local/offline play and authoritative online servers, while clients choose Classic or Reforged presentation independently:

```text
Original/user-owned Darkstone data
             |
             v
   Rust compatibility/import layer
             |
             v
       Canonical game rules
        /             \
       /               \
Local/offline       Server-authoritative
simulation          online simulation
       \               /
        \             /
        canonical world state
              |
      canonical render scene
         /           \
   Classic           Reforged
 presentation       presentation
```

Classic and Reforged are renderer/material/effects policies over the same canonical game state. Offline and online are also execution modes over shared gameplay rules, not separate games. Quests, combat, AI, physics, inventory, scripting, item generation, and character rules remain canonical and reusable by both local and server runtimes.

## 4. Workspace layout

```text
darkstone-rs/
├── Cargo.toml
├── apps/
│   ├── darkstone/
│   ├── darkstone-server/
│   ├── darkstone-realm-gateway/
│   └── darkstone-zone-server/
├── crates/
│   ├── darkstone-core/
│   ├── darkstone-platform/
│   ├── darkstone-assets/
│   ├── darkstone-mtf/
│   ├── darkstone-o3d/
│   ├── darkstone-world/
│   ├── darkstone-script/
│   ├── darkstone-gameplay/
│   ├── darkstone-combat/
│   ├── darkstone-items/
│   ├── darkstone-magic/
│   ├── darkstone-ai/
│   ├── darkstone-quests/
│   ├── darkstone-animation/
│   ├── darkstone-audio/
│   ├── darkstone-input/
│   ├── darkstone-save/
│   ├── darkstone-net/
│   ├── darkstone-protocol/
│   ├── darkstone-party/
│   ├── darkstone-realm/
│   ├── darkstone-zone/
│   ├── darkstone-persistence/
│   ├── darkstone-social/
│   ├── darkstone-render/
│   ├── darkstone-pbr/
│   ├── darkstone-effects/
│   ├── darkstone-post/
│   └── darkstone-ui/
├── shaders/
├── tools/
│   ├── darkstone-inspect/
│   ├── darkstone-extract/
│   ├── darkstone-model-viewer/
│   ├── darkstone-map-viewer/
│   └── darkstone-compat-report/
└── tests/
    ├── fixtures/
    ├── compatibility/
    └── render/
```

The crate boundaries are intentionally narrow so reverse-engineered file formats cannot leak assumptions into gameplay and rendering code.

## 5. Original-data compatibility

### 5.1 Discovery

The launcher/importer may detect common Steam/GOG/native install paths and also accept a user-selected directory. It records only local paths and content fingerprints needed for compatibility diagnostics.

### 5.2 MTF

`darkstone-mtf` owns archive parsing and decompression. Requirements:

- bounds-checked parsing;
- deterministic extraction;
- no unsafe parsing unless isolated and justified;
- malformed-data errors include archive offset and record identity;
- extraction is not required for normal runtime if direct streaming is practical;
- archive rebuilding is deferred until a concrete compatibility need exists.

### 5.3 O3D

`darkstone-o3d` owns model decoding. Initial target is complete static geometry compatibility, followed by animation/skinning/material metadata as discovered.

Decoded data is converted immediately into neutral engine structures. Rendering crates do not read O3D bytes directly.

### 5.4 Unknown formats

Every unrecognized asset type gets:

- stable file identity;
- byte-size and checksum;
- magic/header probe;
- relationship to archive/path;
- compatibility status;
- optional safe hex preview in developer tooling.

No format is guessed silently.

## 6. Game simulation

`darkstone-core` owns deterministic time and world identity. Simulation must be independent of display frame rate.

Baseline design:

- fixed simulation step;
- interpolation for rendering;
- deterministic PRNG streams for gameplay-sensitive procedural behavior;
- explicit entity IDs;
- data-oriented component storage or a small ECS abstraction chosen after profiling;
- serializable world state with versioned schemas.

The engine should prefer behavioral compatibility over perfect instruction-level equivalence.

## 7. World and procedural content

`darkstone-world` reconstructs:

- overworld/settlement scene structure;
- dungeon topology;
- doors, collision, triggers and portals;
- spawn points;
- loot/encounter placement;
- environmental state;
- procedural generation rules.

Generation must be separable from rendering so a generated dungeon can be validated headlessly.

## 8. Combat, items, magic, AI, and quests

These systems remain isolated crates with narrow interfaces.

### Combat

- attack resolution;
- damage types;
- hit/miss/block semantics as discovered;
- status effects;
- death/loot transitions.

### Items

- item identity and templates;
- affixes/stat rolls;
- equipment rules;
- inventory operations;
- vendors and pricing.

### Magic

- spell definition and targeting;
- resource costs;
- projectiles/areas/statuses;
- visual-effect hooks that do not own gameplay outcomes.

### AI

- perception;
- navigation;
- combat intent;
- target choice;
- behavior state.

### Quests

Quest logic is data-driven wherever the original behavior allows it. Scripts operate through a restricted game API rather than direct renderer/platform access.

## 9. Modern Rust-native renderer

### 9.1 Backend

The renderer uses Rust and `wgpu`, with WGSL shaders. Primary native targets:

- Linux: Vulkan;
- Windows: DX12/Vulkan as supported;
- macOS: Metal through wgpu.

The engine negotiates capabilities at runtime and falls back cleanly when optional GPU features are unavailable.

### 9.2 Render graph

The render graph owns dependency/order/resource lifetime for passes including:

1. visibility and GPU scene preparation;
2. shadow maps;
3. depth prepass when profitable;
4. opaque PBR geometry;
5. decals;
6. transparent geometry;
7. volumetrics;
8. particles;
9. water;
10. post-processing;
11. UI/composite.

No gameplay system submits raw GPU commands.

## 10. Reforged material system

The Reforged path uses a metallic/roughness PBR workflow with:

- base color;
- normal map;
- roughness;
- metallic;
- ambient occlusion;
- emissive;
- opacity/cutout;
- optional height/parallax metadata;
- optional clearcoat/subsurface-style material classes only when visually justified.

Original textures can be mapped into conservative enhanced defaults. Generated normal/roughness/AO data is treated as derived local cache data, not as a replacement for authoritative original files.

## 11. Lighting

Reforged mode supports:

- physically meaningful light intensities internally;
- directional sunlight/moonlight;
- point and spot lights;
- shadow-casting torches and spell lights;
- cascaded directional shadows;
- local shadow atlases;
- image-based/environment lighting where appropriate;
- light probes or baked assistance for static interiors if profiling justifies it.

Torch/spell flicker is deterministic visual noise and cannot alter simulation state.

## 12. HDR and post-processing

The render pipeline is HDR internally. Planned effects:

- exposure adaptation;
- filmic tone mapping;
- bloom;
- temporal anti-aliasing or another stable reconstruction path;
- ambient occlusion;
- optional screen-space reflections;
- optional depth of field;
- optional motion blur;
- color grading;
- sharpening at low internal render resolution.

Effects are individually toggleable. Competitive/gameplay readability takes precedence over cinematic effects.

## 13. Volumetrics and atmosphere

Darkstone's dungeons benefit strongly from atmospheric rendering. Reforged mode includes:

- height/distance fog;
- local fog volumes;
- volumetric light shafts;
- fire/smoke contribution;
- weather particles outdoors;
- dust/ash/spore-style environmental particles as scene data permits.

Performance tiers determine froxel resolution and sample count.

## 14. GPU particle system

A compute-driven particle system supports:

- fire;
- embers;
- smoke;
- blood/debris;
- spell trails;
- portals;
- poison/gas;
- weather;
- ambient dust.

Gameplay collision/damage is never delegated to nondeterministic GPU particle simulation. Gameplay owns authoritative hit areas; particles visualize them.

## 15. Water

Water may use:

- depth-aware color/absorption;
- animated normal layers;
- reflection probes or screen-space reflection;
- refraction/distortion;
- foam/shoreline masks where source geometry allows it.

Expensive water features degrade by quality tier.

## 16. Animation and model enhancement

The engine separates original animation semantics from rendered pose quality.

Reforged improvements may include:

- higher-quality interpolation;
- animation blending;
- additive layers;
- foot placement IK;
- aim/look constraints;
- smoother transitions;
- optional high-detail replacement meshes using neutral engine formats.

Replacement models must preserve gameplay attachment points, collision meaning, equipment sockets, and silhouette/readability.

## 17. Classic graphics mode

Classic mode is not a second engine. It is a compatibility presentation preset using:

- original-compatible geometry/material interpretation;
- simplified lighting/shading;
- restrained filtering/effects;
- original-style camera/render behavior where practical;
- optional integer/nearest presentation rules where appropriate.

Classic mode is required for regression comparison against the original appearance and for lower-end systems.

## 18. Camera and display

Both modes support:

- arbitrary desktop resolutions;
- borderless/windowed/fullscreen;
- 16:9, 16:10, ultrawide and sensible aspect correction;
- high-DPI UI;
- high refresh rates;
- frame-rate limits;
- VSync modes supported by the platform;
- render-resolution scaling.

Simulation timing is not tied to refresh rate.

## 19. Performance and scaling

The renderer uses capability-based quality tiers rather than hard-coded vendor checks.

Planned techniques:

- frustum culling;
- hierarchical/occlusion culling when useful;
- GPU instancing;
- LODs;
- texture mip streaming;
- asynchronous asset IO;
- upload staging;
- bounded transient GPU allocations;
- shader/pipeline caching when backend support permits;
- performance telemetry in developer builds.

Initial target: stable 60 FPS at 1080p Reforged settings on a reasonable modern discrete GPU, with higher refresh and 1440p/4K scaling targets established only after real profiling.

## 20. Optional ray tracing

Ray queries/ray-tracing features are optional experimental enhancements only. They cannot be required for:

- correct lighting;
- correct shadows;
- reflections;
- game progression;
- compatibility testing.

Raster/compute paths remain the canonical implementation because backend support is broader and more stable.

## 21. Audio

`darkstone-audio` decodes original audio formats through isolated adapters and provides:

- music;
- UI sounds;
- positional effects;
- environmental ambience;
- voice playback;
- dynamic mixing;
- modern output-device handling.

The renderer never owns audio timing.

## 22. Input

Input supports keyboard/mouse and modern controllers, with:

- full remapping;
- configurable camera controls;
- dead zones and sensitivity;
- accessibility toggles;
- separation between physical input and gameplay actions.

## 23. Saving and migration

The reimplementation uses its own versioned save container unless original-save interoperability proves safe and sufficiently understood.

Requirements:

- atomic writes;
- backups;
- checksum/integrity metadata;
- forward migration functions;
- explicit incompatibility errors rather than corrupt loading.

Original-save import can be added once the format is sufficiently understood and tested.

## 24. Networking, dual-hero parties, and MMO progression

Online functionality is deliberately introduced only after the offline game rebuild is broadly compatible and stable. The sequence is mandatory: basic online co-op first, then two-hero-per-player ownership, then persistence, then the open-world MMO layer.

### 24.1 Shared simulation boundary

The same gameplay crates power offline and online execution. The server runtime owns authoritative simulation when connected online. Clients submit validated player intentions and render replicated state; they do not authoritatively decide combat outcomes, loot, inventory, currency, quest progress, companion AI results, or world ownership.

### 24.2 Online transport and protocol

The preferred transport is QUIC/TLS using a Rust-native implementation such as Quinn, with Tokio for asynchronous server execution. Protocol requirements:

- explicit protocol/schema versions;
- authenticated encrypted sessions;
- reliable streams for login, inventory, quests, trade, chat, party management, and persistence-sensitive events;
- low-latency datagrams or equivalent snapshot traffic for movement and transient world state;
- sequence numbers and replay/duplicate protection where appropriate;
- reconnect and authoritative resynchronization;
- bandwidth/interest management;
- server-side rate validation and abuse limits.

The wire protocol is newly defined for Darkstone-RS and does not depend on obsolete original network services.

### 24.3 Stage 1 online co-op

The first online milestone proves the authoritative model with a conventional party of up to four human players. It is intentionally smaller than the final MMO feature set so networking correctness can be validated before persistence and world distribution are introduced.

Required proof:

- create/join session;
- four human clients;
- authoritative movement/combat/loot;
- latency compensation only where it cannot change authoritative outcomes;
- disconnect/reconnect;
- deterministic headless regression scenarios;
- no client-authoritative inventory or item creation.

### 24.4 Stage 2 — two heroes per human player

After basic co-op passes, every online player receives the same two-character party concept as single player. This becomes a core network ownership invariant:

```text
Human Player Session
        |
        v
     Hero Pair
     /       \
 Hero A     Hero B
```

Rules:

- one human player owns exactly one active two-hero party in normal play;
- either hero may be made the directly controlled primary;
- control can switch between the two heroes without changing ownership;
- the non-primary hero is run by authoritative server-side companion AI using player-selected tactics/commands;
- both heroes retain independent equipment, skills, health/resources, status, and progression;
- party-level state explicitly records shared quest/team ownership where appropriate;
- a standard four-player online group therefore contains eight active heroes;
- companion AI and secondary-hero actions are validated on the server, never trusted from a client simulation.

This ownership model carries unchanged into the MMORPG.

### 24.5 Stage 3 — persistent realm state

Only after dual-hero online play is stable is durable MMO persistence enabled. A relational database such as PostgreSQL, accessed through a Rust layer such as SQLx, stores durable account/world state. Active combat simulation remains in memory and is checkpointed/committed through explicit persistence boundaries.

Persistent records include:

- account identity and entitlements;
- hero pairs and character progression;
- inventory/equipment;
- item provenance;
- skills/spells;
- quests and world progression;
- currency;
- waypoints/discovery;
- friends/guild/party metadata;
- trade and economy audit records;
- realm/zone ownership metadata.

Economy-affecting operations are transactional. Items and currency transfers use server-generated identities/ledgers so duplication and conflicting ownership can be detected.

### 24.6 Stage 4 — persistent open world

The MMORPG world is presented as one continuous realm but simulated by multiple authoritative zone processes. The canonical topology is:

```text
                     Realm Gateway
                          |
                Realm/World Coordinator
                  /       |        \
             Zone A    Zone B     Zone C
                \       |       /
                 Persistent State
```

The realm gateway keeps client session identity stable while zone ownership changes behind it. Neighboring zone data is pre-streamed before boundaries where practical, and authority is transferred explicitly so movement can appear seamless.

Requirements:

- horizontal zone scaling;
- explicit zone ownership leases/epochs;
- seamless or minimally disruptive handoff;
- area-of-interest replication rather than realm-wide broadcasting;
- dynamic population limits and overflow strategy based on measured load;
- crash recovery/checkpoint rules;
- server-side world-event ownership;
- instanced spaces permitted for story dungeons, raids, or load isolation while the overworld remains persistent.

### 24.7 Stage 5 — MMORPG social and large-group play

After persistent-zone operation is stable, the project adds the full MMO layer:

- persistent public realms;
- friends and ignore lists;
- player parties and multi-party raids;
- guilds;
- chat channels and moderation hooks;
- matchmaking/group finder where useful;
- player trade and economy controls;
- world events and bosses;
- larger raid encounters;
- realm population and zone balancing;
- operational/admin tooling;
- telemetry, audit, rollback, and incident recovery procedures.

MMO population is measured in human player sessions/hero pairs. Normal four-player parties remain four humans controlling eight heroes. Larger encounters may combine multiple player parties while preserving the same two-heroes-per-player ownership invariant.

### 24.8 Graphics remain client-selectable

Classic/Reforged presentation is independent of online/offline mode. Valid combinations include:

- Offline + Classic;
- Offline + Reforged;
- Online/MMO + Classic;
- Online/MMO + Reforged.

Dedicated servers are headless and require no GPU renderer. Visual quality settings never affect authoritative simulation.

## 25. Error handling

Libraries return structured errors rather than terminating the process. Key diagnostic context includes:

- archive/path;
- asset ID;
- byte offset;
- subsystem;
- expected vs observed values;
- recoverability.

The retail UI shows concise actionable errors; developer tools expose detailed chains.

## 26. Safety and parser hardening

Original game files are treated as untrusted input.

- checked arithmetic;
- bounds checks before slices/allocations;
- allocation limits;
- recursion/decompression limits;
- fuzzing for MTF/O3D and later binary formats;
- no raw pointer parsing in normal code;
- unsafe blocks, if ever required, are localized and justified.

## 27. Testing strategy

### Unit tests

Each parser/system has deterministic unit tests.

### Golden compatibility tests

User-supplied or redistributable fixture metadata can test:

- archive indexes;
- decoded vertex/index counts;
- model bounds;
- known asset hashes;
- deterministic generation outputs;
- gameplay rule outputs.

Copyrighted original assets are not committed to the public test repository.

### Render tests

Headless/offscreen renders validate:

- material channels;
- shadow correctness;
- camera transforms;
- tone mapping;
- regression signatures.

Pixel-perfect tolerances are used only where backend variance permits; otherwise perceptual/structural metrics are used.

### Fuzzing

Primary fuzz targets begin with MTF and O3D parsers.

### Integration tests

Headless scenarios validate inventory, combat, AI, quest and save behavior without requiring a GPU.

## 28. Tooling milestones

### Milestone 0 — Data compatibility

- find/validate an original install;
- parse and enumerate MTF archives;
- extract/direct-stream assets;
- parse static O3D geometry;
- produce a compatibility report;
- render at least one original static model through the new renderer.

### Milestone 1 — Scene bootstrap

- window/input loop;
- camera;
- asset cache;
- basic world geometry;
- Classic material path;
- Reforged PBR path;
- basic lights/shadows.

### Milestone 2 — Playable simulation slice

- player movement;
- collision;
- one enemy archetype;
- melee/ranged/spell proof;
- loot/inventory proof;
- save/load proof.

### Milestone 3 — Reforged atmosphere

- HDR;
- tone mapping;
- AO;
- bloom;
- volumetric fog;
- particles;
- water;
- quality tiers.

### Milestone 4 — Content compatibility expansion

- additional model/animation formats;
- maps/dungeons;
- quests;
- item/spell database;
- enemy roster;
- audio.

### Milestone 5 — Full campaign compatibility

- campaign progression;
- all major systems;
- save stability;
- broad original-data compatibility.

### Milestone 6 — Authoritative online co-op foundation

- newly defined versioned network protocol;
- encrypted client/server sessions;
- lobby/create/join flow;
- up to four human players;
- authoritative movement, combat, loot, and inventory;
- state snapshots/deltas;
- disconnect/reconnect/resynchronization;
- headless multiplayer regression tests.

### Milestone 7 — Two-hero-per-player online parties

- exactly two active heroes owned by each normal player session;
- primary-hero switching;
- authoritative companion AI for the non-primary hero;
- independent hero equipment/stats/progression;
- four-player/eight-hero standard group;
- bandwidth and AI-load profiling;
- dual-hero multiplayer regression suite.

### Milestone 8 — Persistent accounts and realm state

- account/session service;
- persistent hero-pair progression;
- PostgreSQL-backed durable state;
- transactional inventory/currency/trade operations;
- item provenance/duplication defenses;
- persistence migrations/backups/recovery tests.

### Milestone 9 — Seamless persistent open world

- realm gateway and coordinator;
- authoritative zone servers;
- area-of-interest replication;
- zone pre-streaming and authority handoff;
- dynamic zone population/load management;
- persistent world events;
- instanced dungeon/raid support where useful;
- crash recovery and checkpoint validation.

### Milestone 10 — MMORPG systems and scale

- persistent public realms;
- friends/ignore/party/guild systems;
- chat/moderation hooks;
- trade/economy operational controls;
- group finder or matchmaking where justified;
- large world bosses and multi-party raids;
- realm/zone balancing;
- admin, audit, telemetry, rollback, and incident-recovery tooling;
- load, soak, failure-injection, and security testing.

## 29. Initial technical choices

These are approved unless implementation evidence forces revision:

- Language: Rust stable.
- GPU API abstraction: wgpu.
- Shader language: WGSL.
- Window/event layer: winit unless a concrete blocker appears.
- Math: glam or equivalent lightweight Rust math crate after dependency review.
- Serialization: explicit versioned formats; serde may be used for tooling/configuration, not blindly for binary compatibility formats.
- Logging/diagnostics: tracing-style structured logging.
- Asset hashing: SHA-256 for compatibility reports; faster non-cryptographic hashes may be used internally for transient caches.
- Online transport: QUIC/TLS via a Rust-native implementation such as Quinn unless profiling/interoperability evidence requires a change.
- Async server runtime: Tokio unless profiling evidence requires a different runtime.
- Durable MMO persistence: PostgreSQL through SQLx or an equivalent typed Rust database layer.

## 30. Source research boundary

Public reverse-engineering documentation and independently written format tools may be used as interoperability research inputs. The canonical implementation remains newly written Rust code with its own tests and documentation.

Known public research currently demonstrates:

- full MTF archive decompression;
- static O3D model viewing;
- partial format descriptions.

This validates Milestone 0 feasibility but does not imply that every animation, map, script, save, audio, or network format is already understood.

## 31. Acceptance criteria for the first implementation plan

The first implementation plan is intentionally limited to the initial game rebuild foundation. No online/MMO milestone may be pulled into it. It is accepted only if it produces, in order:

1. a compiling Rust workspace;
2. `darkstone-mtf` with tests and hardened parsing;
3. a compatibility scanner that can enumerate a real user-owned installation without modifying it;
4. `darkstone-o3d` static geometry parsing with tests;
5. a minimal wgpu window/offscreen renderer;
6. one original model rendered from user-owned data in Classic mode;
7. the same neutral model data rendered through a basic Reforged PBR material path;
8. a machine-readable compatibility report;
9. no copyrighted original assets committed or packaged.

## 32. Design invariants

These rules are mandatory throughout the project:

- One canonical Rust engine, not separate Classic/Reforged forks.
- Gameplay state never depends on GPU frame rate.
- GPU visual effects never become authoritative gameplay state.
- Original binary formats terminate at compatibility-layer boundaries.
- Copyrighted original data is user-supplied, never bundled by default.
- Optional graphics features always have functional fallbacks.
- Every discovered binary format receives tests before broad runtime use.
- Parser robustness is a release gate, not cleanup work.
- Reforged graphics must preserve Darkstone's atmosphere and readability rather than chase generic photorealism.
- Offline campaign compatibility through Milestone 5 is a hard gate before online feature implementation.
- Online progression is sequential: authoritative co-op → two-hero ownership → persistence → open-world zoning → full MMO systems.
- Online clients are never authoritative for combat, loot, inventory, currency, quest progress, or companion AI.
- Every normal online player owns a two-hero party after Milestone 7; four humans therefore control eight active heroes.
- The MMO preserves the two-hero-per-player invariant rather than replacing it with one-avatar MMO semantics.
- Classic/Reforged graphics choices remain client-local and never change server simulation.

## 33. Spec self-review

Placeholder scan: PASS — no unresolved placeholder requirements.
Consistency: PASS — Classic/Reforged share one simulation; offline/server modes share canonical gameplay rules; the MMO preserves two-hero ownership.
Scope: PASS as a staged program architecture because implementation is explicitly gated into Milestones 0–10 and the first implementation plan remains limited to the initial rebuild.
Ambiguity: PASS — original assets remain external/user-owned; ray tracing is optional; clients are non-authoritative; online expansion order is mandatory; standard four-player groups become eight-hero groups only after the basic networking milestone is stable.

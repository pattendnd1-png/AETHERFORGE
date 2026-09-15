# AetherAI v0.3.0 Task 11A ChatGPT Import Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Import all recoverable data from an official user-owned ChatGPT export into AetherAI with stable provenance, duplicate-safe re-imports, project reconstruction, attachment bridging, and normal AetherAI retrieval/memory integration.

**Architecture:** Add one focused import boundary that parses an official export into typed external records, then upserts those records through existing AetherAI conversation/project/storage services. Authentication remains outside AetherAI: the desktop launches the official OpenAI/ChatGPT sign-in/export surface and later imports the downloaded archive locally. Imported content never creates a parallel chat runtime.

**Tech Stack:** Rust 1.98 workspace; existing `aether-core`, `aether-storage`, `aether-index`, `aether-retrieval`, `aether-system`, and desktop shell; serde/serde_json/chrono/uuid; only already-vendored archive support after preflight verification.

**Spec:** `docs/superpowers/specs/2026-09-01-aetherai-chatgpt-import-design.md`

## Global Constraints

- Release version remains exactly `0.3.0` until canonical publication.
- Import-all is the canonical v0.3.0 behavior.
- AetherAI never captures or stores OpenAI passwords, phone OTPs, SMS codes, session cookies, browser profiles, or bearer tokens.
- Official OpenAI/ChatGPT login/export surfaces are opened in the system browser.
- No scraping of ChatGPT pages, browser storage, cookies, or undocumented private endpoints.
- Imported source content remains local-first.
- Re-import is deterministic and idempotent.
- External project membership is never fabricated when the export lacks evidence.
- Imported conversation text does not automatically become durable project memory.
- Existing strict Task 10 PASS-artifact rules remain authoritative.
- Existing Task 11 bounded-context and citation rules remain authoritative.
- No new non-vendored crates.io dependency.
- Linux x86_64 and Windows x86_64 remain separate packaging contracts.

---

### Task 11A.1: Establish the Import Crate and Archive Preflight

**Files:**
- Modify: `Cargo.toml`
- Create: `crates/aether-import/Cargo.toml`
- Create: `crates/aether-import/src/lib.rs`
- Create: `crates/aether-import/tests/chatgpt_export_contract.rs`
- Create: `tests/test_v030_chatgpt_import_workspace.sh`

**Interfaces:**
- Consumes: workspace version/dependency policy.
- Produces: `CHATGPT_IMPORT_SCHEMA_VERSION`, `ImportError`, and a verified archive-decoder dependency decision.

- [ ] **Step 1: Write the failing workspace contract**

The shell contract requires the `aether-import` workspace member, crate root, version `0.3.0`, and no `reqwest`, browser-cookie, WebDriver, or credential-capture dependencies.

- [ ] **Step 2: Verify RED**

Run:

```bash
bash tests/test_v030_chatgpt_import_workspace.sh
```

Expected: FAIL because `aether-import` does not exist.

- [ ] **Step 3: Verify native archive support before adding production code**

Use the existing `aether-system` platform boundary. Linux prefers `bsdtar` and falls back to `unzip`; Windows uses PowerShell/.NET ZIP support. List and validate entries before extraction, reject path traversal/link/device entries, and re-validate the extracted tree. Do not add a network dependency.

- [ ] **Step 4: Add the minimal crate**

```rust
#![forbid(unsafe_code)]

pub const CHATGPT_IMPORT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("unsupported ChatGPT export schema: {0}")]
    UnsupportedSchema(String),
    #[error("invalid ChatGPT export: {0}")]
    InvalidExport(String),
    #[error("archive error: {0}")]
    Archive(String),
    #[error("storage error: {0}")]
    Storage(String),
}
```

- [ ] **Step 5: Run workspace contract and crate tests**

Expected: PASS.

---

### Task 11A.2: Define Typed ChatGPT Export Provenance and Stable IDs

**Files:**
- Create: `crates/aether-import/src/chatgpt.rs`
- Modify: `crates/aether-import/src/lib.rs`
- Modify: `crates/aether-storage/src/models.rs`
- Modify: `crates/aether-storage/src/migrations.rs`
- Test: `crates/aether-import/tests/chatgpt_export_contract.rs`
- Test: `crates/aether-storage/tests/v030_chatgpt_import.rs`

**Interfaces:**
- Produces: `ChatGptExportManifest`, `ExternalProject`, `ExternalConversation`, `ExternalMessage`, `ExternalAttachment`, `ExternalSourceKey`.

- [ ] **Step 1: Write stable-key/idempotence tests**

Two parses of the same fixture must produce identical external keys; changed export fingerprints may update records but not allocate duplicate external identities.

- [ ] **Step 2: Verify RED**

Expected: missing typed import records/storage tables.

- [ ] **Step 3: Add normalized external records**

Each normalized record contains source type `ChatGPTExport`, original external ID when available, export fingerprint, source timestamp, and payload fields required by AetherAI. Never synthesize a ChatGPT project ID.

- [ ] **Step 4: Add v3-compatible import provenance tables**

Add tables keyed by `(source_type, external_id)` and export fingerprint. Migration must preserve all existing v0.3 storage.

- [ ] **Step 5: Verify GREEN and migration rollback**

Run import/storage tests and the existing storage migration suite.

---

### Task 11A.3: Parse the Official Export and Import Everything Recoverable

**Files:**
- Create: `crates/aether-import/src/archive.rs`
- Create: `crates/aether-import/src/parser.rs`
- Create: `crates/aether-import/src/fixtures.rs`
- Test: `crates/aether-import/tests/chatgpt_export_contract.rs`

**Interfaces:**
- Produces: `ParsedChatGptExport`.

- [ ] **Step 1: Write fixture tests**

Fixtures cover:
- multiple conversations;
- explicit project metadata;
- chats with no project metadata;
- timestamps/roles;
- attachments present and missing;
- unknown additive fields;
- structurally incompatible schema.

- [ ] **Step 2: Verify RED**

Expected: parser/archive API missing.

- [ ] **Step 3: Implement defensive parsing**

Parse known structures without assuming field ordering. Ignore unknown additive fields. Reject incompatible root shapes with `UnsupportedSchema`. Preserve original IDs and timestamps.

- [ ] **Step 4: Enforce import-all**

No selection filter exists in v0.3.0. Every valid conversation in the chosen export is emitted.

- [ ] **Step 5: Verify GREEN**

Repeated parsing is deterministic.

---

### Task 11A.4: Reconstruct Projects Without Inventing Relationships

**Files:**
- Create: `crates/aether-import/src/reconstruct.rs`
- Test: `crates/aether-import/tests/chatgpt_project_reconstruction.rs`

**Interfaces:**
- Consumes: `ParsedChatGptExport`.
- Produces: `ReconstructedImport`.

- [ ] **Step 1: Write explicit-project and unassigned tests**

Explicit external project IDs retain membership. A conversation lacking project evidence remains unassigned and is not attached to a fabricated external project.

- [ ] **Step 2: Verify RED**

- [ ] **Step 3: Implement reconstruction**

Use only explicit export relationships. UI grouping labels are display metadata, never external project identity.

- [ ] **Step 4: Verify GREEN**

Require deterministic ordering by original timestamp then external ID.

---

### Task 11A.5: Transactional Upsert into AetherAI Conversation/Project Storage

**Files:**
- Create: `crates/aether-import/src/service.rs`
- Modify: `crates/aether-storage/src/lib.rs`
- Modify: `crates/aether-storage/src/models.rs`
- Test: `crates/aether-import/tests/chatgpt_import_service.rs`
- Test: `crates/aether-storage/tests/v030_chatgpt_import.rs`

**Interfaces:**
- Produces: `ChatGptImportService::import_all` and `ImportSummary`.

`ImportSummary` reports projects/chats/messages/files as added, updated, unchanged, unavailable, and failed.

- [ ] **Step 1: Write same-export re-import test**

Import fixture twice. Second import must report no duplicates.

- [ ] **Step 2: Write newer-export merge test**

New messages are added; existing imported messages retain the same AetherAI identity; local AetherAI-authored messages remain untouched.

- [ ] **Step 3: Verify RED**

- [ ] **Step 4: Implement transactional batch upsert**

Use stable external keys and bounded transactions. Cancellation stops future batches without corrupting committed records.

- [ ] **Step 5: Verify GREEN**

---

### Task 11A.6: Bridge Attachments, PASS Checkpoints, Retrieval, and Memory

**Files:**
- Create: `crates/aether-import/src/attachments.rs`
- Modify: `crates/aether-retrieval/src/lib.rs`
- Create: `crates/aether-retrieval/src/imported_chat.rs`
- Modify: `crates/aether-index/src/lib.rs`
- Test: `crates/aether-import/tests/chatgpt_attachment_bridge.rs`
- Test: `crates/aether-retrieval/tests/v030_imported_chat.rs`

**Interfaces:**
- Produces: typed imported-chat retrieval source and attachment bridge.

- [ ] **Step 1: Write missing/present attachment tests**

Present file bytes enter the attachment registry. Missing attachment references remain unavailable records.

- [ ] **Step 2: Write PASS-safety tests**

A chat message containing `AETHERAI_V0_3_0_VERIFY=PASS` does not create checkpoint memory. An actual imported eligible verification file does.

- [ ] **Step 3: Write imported-chat retrieval tests**

Retrieved imported messages expose ChatGPT provenance and never masquerade as filesystem citations.

- [ ] **Step 4: Verify RED**

- [ ] **Step 5: Implement bridge and typed retrieval source**

All model injection still passes through Task 11 bounded context.

- [ ] **Step 6: Verify GREEN**

Run index, retrieval, storage, and import suites.

---

### Task 11A.7: Add Desktop Connect/Import-All UX Without Credential Capture

**Files:**
- Modify: `apps/aetherai-desktop` integration/settings modules discovered by Task 11A preflight
- Modify: `crates/aether-system` browser/open-path boundary if required
- Test: desktop/system tests at the discovered live paths

**Interfaces:**
- Produces: `Connect ChatGPT`, `Import ChatGPT Export`, import progress, and re-import summary.

- [ ] **Step 1: Preflight exact desktop/settings/system paths**

Do not guess desktop module names. Record the exact existing browser-launch and file-picker/system integration surfaces.

- [ ] **Step 2: Write auth-boundary test**

The button requests an official OpenAI/ChatGPT URL through system-browser integration. No password, phone OTP, cookie, or token input/storage fields exist in AetherAI.

- [ ] **Step 3: Write import-all UI service test**

Selecting one official export invokes `ChatGptImportService::import_all` with no per-chat selection filter.

- [ ] **Step 4: Verify RED**

- [ ] **Step 5: Implement minimal DragonGlass UI**

Show import counts/status/errors and provenance. Do not add a separate Work mode.

- [ ] **Step 6: Verify GREEN**

Run desktop release build and system tests.

---

### Task 11A.8: Add Verification Gates and Uploadable Handoff

**Files:**
- Modify: `xtask/src/main.rs`
- Create: `tests/test_v030_chatgpt_import_contract.sh`
- Modify: `README.md`
- Modify: `CHANGELOG.md`
- Generate: `AetherAI-v0.3.0-TASK11A-VERIFY.txt`

**Interfaces:**
- Produces final Task 11A machine-readable gates.

- [ ] **Step 1: Write verifier-report RED test**

Final report must include the Task 11A endpoints and end with `AETHERAI_V0_3_0_TASK11A=PASS`.

- [ ] **Step 2: Add exact gates**

```text
AETHERAI_CHATGPT_IMPORT_AUTH_BOUNDARY
AETHERAI_CHATGPT_IMPORT_ALL
AETHERAI_CHATGPT_IMPORT_IDEMPOTENT
AETHERAI_CHATGPT_IMPORT_PROVENANCE
AETHERAI_CHATGPT_IMPORT_PROJECT_RECONSTRUCTION
AETHERAI_CHATGPT_IMPORT_NO_FAKE_PROJECTS
AETHERAI_CHATGPT_IMPORT_ATTACHMENT_BRIDGE
AETHERAI_CHATGPT_IMPORT_PASS_CHECKPOINT_BRIDGE
AETHERAI_CHATGPT_IMPORT_RETRIEVAL
AETHERAI_CHATGPT_IMPORT_MEMORY_SAFETY
AETHERAI_CHATGPT_IMPORT_NO_CREDENTIAL_STORAGE
```

- [ ] **Step 3: Run full regression**

Run import, chat, retrieval, index, storage, desktop/system, workspace check, fmt, and Clippy with warnings denied.

- [ ] **Step 4: Write uploadable verification**

Expected endpoint:

```text
AETHERAI_V0_3_0_TASK11A=PASS
```

## Plan Self-Review

- Spec coverage: import-all, official auth boundary, local storage, idempotence, provenance, no fake project relationships, attachments, Task 10 checkpoints, Task 11 bounded retrieval, memory safety, desktop UX, and verification are mapped to Tasks 11A.1-11A.8.
- Placeholder scan: no `TBD`, `TODO`, or deferred implementation markers.
- Type consistency: the import crate owns external parsing/provenance; existing AetherAI crates remain authoritative for local conversations, attachments, indexing, retrieval, memory, and desktop/system integration.

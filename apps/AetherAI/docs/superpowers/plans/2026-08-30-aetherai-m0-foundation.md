# AetherAI M0 Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build AetherAI v0.1.0 as a runnable, testable Rust foundation with one unified chat/workspace model, a stable model-provider interface, transparent processing controls, SQLite persistence, a free mock inference provider, a CLI chat loop, and a Rust verification harness.

**Architecture:** M0 is deliberately small and vertical. `aether-core` owns durable IDs/settings/domain primitives; `aether-model-api` owns provider contracts; `aether-chat` owns conversation/workspace behavior; `aether-storage` persists those domain objects in SQLite; `aetherai-cli` composes the crates into a usable executable; `xtask` performs host verification and emits the version-specific verification report. No real model runtime is introduced until M1, so M0 remains zero-download and deterministic.

**Tech Stack:** Rust 2024 edition; Tokio; async-trait; serde/serde_json; thiserror; uuid; chrono; rusqlite with bundled SQLite; futures-core/futures-util; clap; tempfile for tests.

**Spec:** `docs/superpowers/specs/2026-08-30-aetherai-design.md`

## Global Constraints

- Product name is `AetherAI`.
- Initial canonical version is `0.1.0`; one canonical artifact per version and no RC/revision suffixes.
- Core runtime must remain 100% free and must not require paid APIs, usage credits, subscriptions, or a hosted backend.
- First-party runtime code is Rust-first and local-first.
- There is one chat system: every conversation may gain persistent workspace state without moving to a separate Work mode.
- Model execution must be isolated behind a stable provider trait; M0 uses only a free deterministic mock provider.
- Processing presets and advanced controls are explicit user-visible configuration, never hidden behavior.
- Persistent state defaults to local SQLite/filesystem storage.
- Verification must produce `AetherAI-v0.1.0-VERIFY.txt` suitable for uploading back into a development chat.
- Linux handoff commands target `~/Downloads`.

---

### Task 1: Establish the Rust Workspace and Core Domain Contract

**Files:**
- Create: `Cargo.toml`
- Create: `.gitignore`
- Create: `README.md`
- Create: `crates/aether-core/Cargo.toml`
- Create: `crates/aether-core/src/lib.rs`
- Test: `crates/aether-core/src/lib.rs` unit tests

**Interfaces:**
- Consumes: none.
- Produces: `ConversationId`, `MessageId`, `WorkspaceId`, `Role`, `Message`, `ConversationKind`, `ProcessingPreset`, `GenerationSettings`, `ConversationSettings`, `WorkspaceState`, `Conversation`, and `AetherCoreError`.

- [ ] **Step 1: Write the failing core-domain tests**

Add tests first in `crates/aether-core/src/lib.rs` that express the permanent M0 invariants:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn balanced_preset_is_transparent_and_overridable() {
        let mut settings = GenerationSettings::from_preset(ProcessingPreset::Balanced);
        assert_eq!(settings.max_output_tokens, 2048);
        assert_eq!(settings.temperature, 0.7);
        settings.temperature = 0.2;
        assert_eq!(settings.temperature, 0.2);
    }

    #[test]
    fn ordinary_chat_can_gain_workspace_state_without_changing_identity() {
        let mut chat = Conversation::new("Test chat");
        let id = chat.id;
        assert!(chat.workspace.is_none());
        chat.ensure_workspace("Test Workspace");
        assert_eq!(chat.id, id);
        assert!(chat.workspace.is_some());
    }

    #[test]
    fn generation_settings_reject_invalid_sampling_ranges() {
        let mut settings = GenerationSettings::from_preset(ProcessingPreset::Balanced);
        settings.temperature = -0.1;
        assert!(matches!(settings.validate(), Err(AetherCoreError::InvalidTemperature(_))));
    }
}
```

- [ ] **Step 2: Run the focused test and confirm the expected compile failure**

Run:

```bash
cargo test -p aether-core
```

Expected: FAIL because the workspace/crate/domain types do not exist yet.

- [ ] **Step 3: Create the workspace manifest and core crate**

Create root `Cargo.toml`:

```toml
[workspace]
resolver = "2"
members = [
    "apps/aetherai-cli",
    "crates/aether-core",
    "crates/aether-model-api",
    "crates/aether-chat",
    "crates/aether-storage",
    "xtask",
]

[workspace.package]
version = "0.1.0"
edition = "2024"
license = "AGPL-3.0-or-later"
rust-version = "1.85"

[workspace.dependencies]
async-trait = "0.1"
chrono = { version = "0.4", features = ["serde"] }
clap = { version = "4", features = ["derive"] }
futures-core = "0.3"
futures-util = "0.3"
rusqlite = { version = "0.32", features = ["bundled", "chrono", "uuid"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tempfile = "3"
thiserror = "2"
tokio = { version = "1", features = ["macros", "rt-multi-thread", "io-util", "io-std", "sync"] }
uuid = { version = "1", features = ["v4", "serde"] }
```

Create `crates/aether-core/Cargo.toml` with workspace dependencies on `chrono`, `serde`, `thiserror`, and `uuid`.

Implement these exact public types in `crates/aether-core/src/lib.rs`:

```rust
pub type ConversationId = uuid::Uuid;
pub type MessageId = uuid::Uuid;
pub type WorkspaceId = uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Role { System, Developer, User, Assistant, Tool }

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Message {
    pub id: MessageId,
    pub role: Role,
    pub content: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ConversationKind { Chat, Coding, Research, Project, Document, Analysis, AgentTask }

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ProcessingPreset { Fast, Balanced, Deep, Maximum }

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GenerationSettings {
    pub preset: ProcessingPreset,
    pub max_output_tokens: u32,
    pub temperature: f32,
    pub top_p: f32,
    pub top_k: u32,
    pub seed: Option<u64>,
    pub context_tokens: u32,
    pub cpu_threads: Option<u16>,
    pub gpu_layers: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ConversationSettings {
    pub model: String,
    pub generation: GenerationSettings,
    pub memory_enabled: bool,
    pub web_enabled: bool,
    pub code_execution_enabled: bool,
    pub agent_depth: u8,
    pub concurrent_subagents: u8,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WorkspaceState {
    pub id: WorkspaceId,
    pub name: String,
    pub root: Option<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Conversation {
    pub id: ConversationId,
    pub title: String,
    pub kind: ConversationKind,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub settings: ConversationSettings,
    pub workspace: Option<WorkspaceState>,
    pub messages: Vec<Message>,
}
```

Implement `GenerationSettings::from_preset`, `GenerationSettings::validate`, `ConversationSettings::default`, `Conversation::new`, `Conversation::ensure_workspace`, and `Conversation::push_message`. `Balanced` must default to 2048 max output tokens, temperature `0.7`, top-p `0.95`, top-k `40`, context `8192`; other presets should be explicit and deterministic.

Define typed validation failures:

```rust
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum AetherCoreError {
    #[error("temperature must be between 0.0 and 2.0, got {0}")]
    InvalidTemperature(f32),
    #[error("top_p must be between 0.0 and 1.0, got {0}")]
    InvalidTopP(f32),
    #[error("max_output_tokens must be greater than zero")]
    ZeroOutputTokens,
    #[error("context_tokens must be greater than zero")]
    ZeroContextTokens,
}
```

- [ ] **Step 4: Add repository hygiene and minimal README**

Create `.gitignore` containing at minimum:

```text
/target/
*.db
*.db-shm
*.db-wal
AetherAI-v*-VERIFY.txt
AetherAI-v*.tar.gz
AetherAI-v*-SHA256SUMS.txt
```

Create `README.md` identifying AetherAI v0.1.0, its AGPL-3.0-or-later baseline, its zero-paid-API goal, and the M0 scope. Do not claim real local-model inference is present until M1.

- [ ] **Step 5: Run core tests and formatting**

Run:

```bash
cargo fmt --all --check
cargo test -p aether-core
```

Expected: both PASS.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml .gitignore README.md crates/aether-core
git commit -m "feat: establish AetherAI core domain"
```

---

### Task 2: Define the Stable Model Provider API and Deterministic Mock Provider

**Files:**
- Create: `crates/aether-model-api/Cargo.toml`
- Create: `crates/aether-model-api/src/lib.rs`
- Test: `crates/aether-model-api/src/lib.rs` unit tests

**Interfaces:**
- Consumes: `aether_core::{GenerationSettings, Message}`.
- Produces: `ProviderCapabilities`, `ModelDescriptor`, `GenerationRequest`, `GenerationChunk`, `EmbeddingRequest`, `EmbeddingResponse`, `Tokenization`, `ProviderHealth`, `ModelProvider`, `MockProvider`, `ModelError`.

- [ ] **Step 1: Write provider contract tests first**

```rust
#[tokio::test]
async fn mock_provider_reports_free_local_capabilities() {
    let provider = MockProvider::default();
    let caps = provider.capabilities().await;
    assert!(caps.text_generation);
    assert!(!caps.requires_network);
    assert!(!caps.requires_paid_service);
}

#[tokio::test]
async fn mock_provider_generation_is_deterministic() {
    let provider = MockProvider::default();
    let request = GenerationRequest::single_user("hello");
    let chunks = provider.generate(request).await.unwrap();
    assert_eq!(chunks, vec![GenerationChunk::Text("AetherAI mock: hello".into())]);
}
```

- [ ] **Step 2: Run the focused test and verify failure**

Run:

```bash
cargo test -p aether-model-api
```

Expected: FAIL because the provider crate is not yet implemented.

- [ ] **Step 3: Implement provider types and trait**

Use owned deterministic chunks in M0 instead of a live stream so the contract is simple; M1 may introduce streaming behind an additive method without leaking runtime-specific types.

Implement:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderCapabilities {
    pub text_generation: bool,
    pub embeddings: bool,
    pub tool_calling: bool,
    pub vision: bool,
    pub requires_network: bool,
    pub requires_paid_service: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelDescriptor {
    pub id: String,
    pub display_name: String,
    pub context_tokens: u32,
    pub local: bool,
}

#[derive(Debug, Clone)]
pub struct GenerationRequest {
    pub model: String,
    pub messages: Vec<aether_core::Message>,
    pub settings: aether_core::GenerationSettings,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenerationChunk { Text(String) }

#[derive(Debug, Clone)]
pub struct EmbeddingRequest { pub input: Vec<String> }

#[derive(Debug, Clone, PartialEq)]
pub struct EmbeddingResponse { pub vectors: Vec<Vec<f32>> }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tokenization { pub token_count: usize }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderHealth { Ready, Degraded(String), Unavailable(String) }

#[derive(Debug, thiserror::Error)]
pub enum ModelError {
    #[error("model provider error: {0}")]
    Provider(String),
    #[error("embeddings are unsupported by this provider")]
    EmbeddingsUnsupported,
}

#[async_trait::async_trait]
pub trait ModelProvider: Send + Sync {
    async fn capabilities(&self) -> ProviderCapabilities;
    async fn models(&self) -> Result<Vec<ModelDescriptor>, ModelError>;
    async fn generate(&self, request: GenerationRequest) -> Result<Vec<GenerationChunk>, ModelError>;
    async fn embed(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse, ModelError>;
    async fn tokenize(&self, input: &str) -> Result<Tokenization, ModelError>;
    async fn health(&self) -> ProviderHealth;
}
```

`GenerationRequest::single_user` must use `model = "aetherai/mock"` and the balanced preset. `MockProvider::generate` must concatenate the final user message into exactly `AetherAI mock: {content}`. It must report `requires_network=false` and `requires_paid_service=false`.

- [ ] **Step 4: Run provider tests and lint**

Run:

```bash
cargo test -p aether-model-api
cargo clippy -p aether-model-api --all-targets -- -D warnings
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/aether-model-api
git commit -m "feat: add model provider contract"
```

---

### Task 3: Implement Unified Chat/Workspace Service

**Files:**
- Create: `crates/aether-chat/Cargo.toml`
- Create: `crates/aether-chat/src/lib.rs`
- Test: `crates/aether-chat/src/lib.rs` unit tests

**Interfaces:**
- Consumes: `aether_core::{Conversation, ConversationId, Message, Role}` and `aether_model_api::{GenerationChunk, GenerationRequest, ModelProvider}`.
- Produces: `ChatService<P: ModelProvider>`, `ChatError`, `ChatTurnResult`.

- [ ] **Step 1: Write chat orchestration tests first**

```rust
#[tokio::test]
async fn send_user_message_appends_user_and_assistant_turns() {
    let provider = MockProvider::default();
    let mut service = ChatService::new(provider);
    let mut conversation = Conversation::new("Hello");

    let result = service.send_user_message(&mut conversation, "ping").await.unwrap();

    assert_eq!(result.assistant_text, "AetherAI mock: ping");
    assert_eq!(conversation.messages.len(), 2);
    assert_eq!(conversation.messages[0].role, Role::User);
    assert_eq!(conversation.messages[1].role, Role::Assistant);
}

#[tokio::test]
async fn provider_failure_does_not_append_a_fake_assistant_message() {
    let mut conversation = Conversation::new("Failure");
    let provider = FailingProvider;
    let mut service = ChatService::new(provider);

    assert!(service.send_user_message(&mut conversation, "ping").await.is_err());
    assert_eq!(conversation.messages.len(), 1);
    assert_eq!(conversation.messages[0].role, Role::User);
}
```

Define the test-only `FailingProvider` in the test module and implement the full trait with `generate` returning `Err(ModelError::Provider("forced".into()))`.

- [ ] **Step 2: Run tests and verify failure**

Run:

```bash
cargo test -p aether-chat
```

Expected: FAIL because `ChatService` does not exist.

- [ ] **Step 3: Implement the orchestration service**

Implement:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatTurnResult {
    pub assistant_text: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ChatError {
    #[error(transparent)]
    InvalidSettings(#[from] aether_core::AetherCoreError),
    #[error(transparent)]
    Model(#[from] aether_model_api::ModelError),
    #[error("provider returned no assistant text")]
    EmptyResponse,
}

pub struct ChatService<P> {
    provider: P,
}

impl<P: aether_model_api::ModelProvider> ChatService<P> {
    pub fn new(provider: P) -> Self;

    pub async fn send_user_message(
        &mut self,
        conversation: &mut aether_core::Conversation,
        content: impl Into<String>,
    ) -> Result<ChatTurnResult, ChatError>;
}
```

The method must validate generation settings before provider execution, append the user message, construct `GenerationRequest` from the full transcript and current per-chat settings, call the provider, join all `GenerationChunk::Text` values, reject an empty response, then append exactly one assistant message. On provider failure, preserve the user message but do not append assistant content.

- [ ] **Step 4: Run tests and lint**

Run:

```bash
cargo test -p aether-chat
cargo clippy -p aether-chat --all-targets -- -D warnings
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/aether-chat
git commit -m "feat: add unified chat orchestration"
```

---

### Task 4: Add Transactional SQLite Persistence for Chats and Workspace State

**Files:**
- Create: `crates/aether-storage/Cargo.toml`
- Create: `crates/aether-storage/src/lib.rs`
- Test: `crates/aether-storage/src/lib.rs` integration-style unit tests using `tempfile`

**Interfaces:**
- Consumes: `aether_core::{Conversation, ConversationId}` serialized as JSON payloads.
- Produces: `SqliteStore`, `StorageError`, schema version `1`, methods `open`, `migrate`, `save_conversation`, `load_conversation`, `list_conversations`.

- [ ] **Step 1: Write persistence round-trip tests first**

```rust
#[test]
fn conversation_round_trip_preserves_workspace_and_settings() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("aetherai.db");
    let store = SqliteStore::open(&db).unwrap();

    let mut conversation = Conversation::new("Persistent");
    conversation.ensure_workspace("Workspace");
    conversation.settings.generation.temperature = 0.25;
    store.save_conversation(&conversation).unwrap();

    let loaded = store.load_conversation(conversation.id).unwrap().unwrap();
    assert_eq!(loaded, conversation);
}

#[test]
fn reopening_database_keeps_saved_chat() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("aetherai.db");
    let id = {
        let store = SqliteStore::open(&db).unwrap();
        let conversation = Conversation::new("Reload me");
        let id = conversation.id;
        store.save_conversation(&conversation).unwrap();
        id
    };

    let reopened = SqliteStore::open(&db).unwrap();
    assert!(reopened.load_conversation(id).unwrap().is_some());
}
```

- [ ] **Step 2: Run tests and verify failure**

Run:

```bash
cargo test -p aether-storage
```

Expected: FAIL because the storage crate does not exist.

- [ ] **Step 3: Implement schema v1 and storage API**

Use a compact schema in M0:

```sql
CREATE TABLE IF NOT EXISTS schema_meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS conversations (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    payload_json TEXT NOT NULL
);
```

`SqliteStore::open(path)` must open SQLite, call `migrate`, and return the store. `migrate` must run in a transaction and set `schema_version=1`. `save_conversation` must use an upsert inside a transaction. `load_conversation` must deserialize the payload and validate that the payload ID matches the requested row ID. `list_conversations` returns decoded conversations ordered by `updated_at DESC`.

Define:

```rust
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("stored conversation id mismatch: row={row_id}, payload={payload_id}")]
    IdMismatch { row_id: String, payload_id: String },
}
```

- [ ] **Step 4: Run persistence tests and clippy**

Run:

```bash
cargo test -p aether-storage
cargo clippy -p aether-storage --all-targets -- -D warnings
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/aether-storage
git commit -m "feat: persist unified chats in sqlite"
```

---

### Task 5: Build the AetherAI CLI Vertical Slice

**Files:**
- Create: `apps/aetherai-cli/Cargo.toml`
- Create: `apps/aetherai-cli/src/main.rs`
- Create: `apps/aetherai-cli/src/commands.rs`
- Test: `apps/aetherai-cli/src/commands.rs` unit tests

**Interfaces:**
- Consumes: `aether_core`, `aether_chat::ChatService`, `aether_model_api::MockProvider`, `aether_storage::SqliteStore`.
- Produces: binary `aetherai`, command parser for `/help`, `/preset`, `/workspace`, `/settings`, `/quit`, and an interactive chat loop.

- [ ] **Step 1: Write command parsing tests first**

```rust
#[test]
fn parses_processing_preset_command() {
    assert_eq!(parse_command("/preset deep"), Some(Command::Preset(ProcessingPreset::Deep)));
}

#[test]
fn parses_workspace_command_with_name() {
    assert_eq!(parse_command("/workspace Forge Lab"), Some(Command::Workspace("Forge Lab".into())));
}

#[test]
fn ordinary_text_is_not_a_command() {
    assert_eq!(parse_command("hello AetherAI"), None);
}
```

- [ ] **Step 2: Run CLI tests and verify failure**

Run:

```bash
cargo test -p aetherai-cli
```

Expected: FAIL because the CLI crate is not implemented.

- [ ] **Step 3: Implement command parser and interactive runtime**

`commands.rs` must define:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Help,
    Preset(aether_core::ProcessingPreset),
    Workspace(String),
    Settings,
    Quit,
}

pub fn parse_command(input: &str) -> Option<Command>;
```

`main.rs` must use `clap` with:

```text
AetherAI 0.1.0
Usage: aetherai [--data-dir PATH] [--new TITLE]
```

Default data directory: `$XDG_DATA_HOME/aetherai` when set, otherwise `$HOME/.local/share/aetherai`; if neither is available, use `./.aetherai`.

On launch, create/open `aetherai.db`, create a new conversation, save it, and print:

```text
AetherAI v0.1.0 — free local foundation
Provider: aetherai/mock
Type /help for commands.
```

Input behavior:

- normal text: call `ChatService<MockProvider>`, persist conversation, print `AetherAI> {assistant_text}`;
- `/preset fast|balanced|deep|maximum`: update the per-chat generation preset using `GenerationSettings::from_preset`, persist, print the chosen preset;
- `/workspace NAME`: call `ensure_workspace`, persist, confirm that the same chat now has workspace state;
- `/settings`: print model, preset, context tokens, max output tokens, temperature, top-p, top-k, optional CPU threads, optional GPU layers, memory/web/code toggles, agent depth, concurrent subagents;
- `/help`: print command help;
- `/quit`: save and exit 0.

Do not add paid/cloud provider configuration in M0.

- [ ] **Step 4: Run CLI tests and a scripted smoke session**

Run:

```bash
cargo test -p aetherai-cli
printf 'hello\n/preset deep\n/workspace Test Lab\n/settings\n/quit\n' | cargo run -q -p aetherai-cli -- --data-dir /tmp/aetherai-m0-smoke --new "Smoke"
```

Expected output must include:

```text
AetherAI mock: hello
Processing preset: Deep
Workspace enabled: Test Lab
```

- [ ] **Step 5: Commit**

```bash
git add apps/aetherai-cli
git commit -m "feat: add AetherAI CLI vertical slice"
```

---

### Task 6: Add Rust-Native Verification, Release Metadata, and Canonical v0.1.0 Artifacts

**Files:**
- Create: `xtask/Cargo.toml`
- Create: `xtask/src/main.rs`
- Create: `CHANGELOG.md`
- Create: `LICENSE`
- Modify: `README.md`
- Generated: `AetherAI-v0.1.0-VERIFY.txt`
- Generated: `AetherAI-v0.1.0-SHA256SUMS.txt`
- Generated: `AetherAI-v0.1.0.tar.gz`

**Interfaces:**
- Consumes: workspace Cargo metadata and host commands.
- Produces: `cargo run -p xtask -- verify`, verification report, source archive, checksums, canonical PASS endpoint `AETHERAI_V0_1_0_VERIFY=PASS`.

- [ ] **Step 1: Write verifier unit tests first**

In `xtask/src/main.rs`, isolate report formatting and test it:

```rust
#[test]
fn report_ends_with_canonical_pass_endpoint() {
    let report = render_report(&[
        CheckResult::pass("FMT"),
        CheckResult::pass("CLIPPY"),
        CheckResult::pass("TEST"),
        CheckResult::pass("BUILD"),
    ]);
    assert!(report.ends_with("AETHERAI_V0_1_0_VERIFY=PASS\n"));
}

#[test]
fn any_failed_check_makes_report_fail() {
    let report = render_report(&[CheckResult::fail("TEST", 101)]);
    assert!(report.ends_with("AETHERAI_V0_1_0_VERIFY=FAIL\n"));
}
```

- [ ] **Step 2: Run verifier tests and verify failure**

Run:

```bash
cargo test -p xtask
```

Expected: FAIL because `xtask` does not yet exist.

- [ ] **Step 3: Implement `xtask verify`**

`xtask` must run these checks from the repository root and capture only concise status lines in the report:

```text
AETHERAI_VERSION=0.1.0
AETHERAI_FMT=PASS|FAIL:<code>
AETHERAI_CLIPPY=PASS|FAIL:<code>
AETHERAI_TEST=PASS|FAIL:<code>
AETHERAI_BUILD=PASS|FAIL:<code>
AETHERAI_CLI_SMOKE=PASS|FAIL:<code>
AETHERAI_V0_1_0_VERIFY=PASS|FAIL
```

Commands:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace --release
```

For `AETHERAI_CLI_SMOKE`, launch the release CLI with a temporary data directory, pipe `hello\n/quit\n`, and require stdout to contain `AetherAI mock: hello`.

Write the report to `AetherAI-v0.1.0-VERIFY.txt` in the repository root. Exit 0 only if every check passes.

- [ ] **Step 4: Add release docs and license**

`CHANGELOG.md` must record v0.1.0 M0 features only: Rust workspace, core domain, unified chat/workspace state, processing controls, provider trait/mock provider, SQLite persistence, CLI, verifier. `README.md` must include this one-line Linux verification command:

```bash
cd "$HOME/Downloads/AetherAI" && cargo run -q -p xtask -- verify
```

Use the canonical AGPL-3.0-or-later license text in `LICENSE`.

- [ ] **Step 5: Run complete verification**

Run:

```bash
cargo run -p xtask -- verify
```

Expected final line in `AetherAI-v0.1.0-VERIFY.txt`:

```text
AETHERAI_V0_1_0_VERIFY=PASS
```

- [ ] **Step 6: Create canonical source archive and checksum file**

From the parent directory, archive tracked source only:

```bash
git archive --format=tar.gz --prefix=AetherAI-v0.1.0/ -o AetherAI-v0.1.0.tar.gz HEAD
sha256sum AetherAI-v0.1.0.tar.gz AetherAI-v0.1.0-VERIFY.txt > AetherAI-v0.1.0-SHA256SUMS.txt
```

The checksum file must contain exactly the archive and verification report entries for M0.

- [ ] **Step 7: Commit release metadata and verifier**

```bash
git add xtask CHANGELOG.md LICENSE README.md
git commit -m "release: verify AetherAI v0.1.0 foundation"
```

- [ ] **Step 8: Recreate archive from the final release commit and re-run checksums**

```bash
git archive --format=tar.gz --prefix=AetherAI-v0.1.0/ -o AetherAI-v0.1.0.tar.gz HEAD
sha256sum AetherAI-v0.1.0.tar.gz AetherAI-v0.1.0-VERIFY.txt > AetherAI-v0.1.0-SHA256SUMS.txt
```

Expected deliverables:

```text
AetherAI-v0.1.0.tar.gz
AetherAI-v0.1.0-VERIFY.txt
AetherAI-v0.1.0-SHA256SUMS.txt
```

---

## Plan Self-Review Results

- M0 scope covers every item in Architecture §24 M0: workspace, core types, unified chat/workspace storage, provider trait, mock provider, CLI, SQLite, configuration, verification harness.
- Processing controls from §7 are represented at M0 as presets plus persisted advanced generation fields; unsupported future runtime-specific controls are not falsely claimed as implemented.
- No paid/cloud provider is introduced.
- No placeholder crates for later milestones are created.
- Provider, chat, and persistence boundaries are independently testable.
- Verification has one explicit canonical PASS endpoint and a version-specific uploadable `.txt` artifact.

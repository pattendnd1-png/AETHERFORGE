# AetherAI v0.3.0 Task 11A — ChatGPT Account Import & Project Reconstruction Design

## Goal

Import the user's complete recoverable ChatGPT account export into AetherAI as local, provenance-preserving project/chat history that participates in the same conversation, retrieval, citation, memory, and verification systems as local projects.

## Authentication and account ownership

AetherAI exposes **Connect ChatGPT**, but it does not implement an OpenAI password, phone-number, OTP, or cookie capture form.

The connection flow launches the system browser to the official OpenAI/ChatGPT account sign-in/export surface. Email, phone-number, federated, or SSO authentication is handled only by OpenAI's official surface when offered for that account. AetherAI never receives or persists the user's OpenAI password, SMS code, one-time code, session cookie, browser profile, or bearer token.

Until OpenAI exposes a supported third-party API for full ChatGPT account-history export, AetherAI imports an official user-requested export file. It does not scrape ChatGPT pages, private endpoints, browser storage, or cookies.

## Import-all behavior

Selecting a ChatGPT export imports every recoverable conversation and project relationship in that export. Selective import may be added later, but import-all is the canonical v0.3.0 behavior.

The importer preserves:

- original external conversation/project/message identifiers when present;
- original timestamps and roles;
- conversation titles;
- project membership/instructions when explicitly represented by the export;
- file/attachment records when bytes or usable references are present;
- source-export SHA-256 and import timestamp;
- source type `ChatGPTExport`.

The importer must not invent project membership. If the export does not contain enough metadata to prove that chats belonged to a specific ChatGPT Project, those chats are retained as imported ChatGPT conversations with explicit unassigned provenance.

## Local-first storage

The official export is read locally. Parsed data is written into AetherAI's existing SQLite/project stores. No imported content is uploaded to another service by the importer.

The raw export archive is not duplicated into AetherAI durable storage by default. AetherAI stores the archive fingerprint and normalized imported records. A user may retain or delete the original export independently.

## Idempotence and incremental re-import

Each import receives a deterministic export fingerprint from the archive bytes or canonical extracted source manifest.

Normalized records use stable external-source keys such as:

`ChatGPTExport + external_id`

Re-importing the same export does not duplicate projects, conversations, messages, files, or memory. A newer export updates known records and adds newly observed records. Existing local AetherAI messages are never overwritten by an imported external record.

## Retrieval and citations

Imported conversations participate in AetherAI's normal retrieval pipeline. They remain distinguishable from filesystem sources.

Filesystem-backed imported attachments use normal `LocalCitation` path/range/hash citations after indexing.

Imported conversation/message sources use typed ChatGPT-import provenance rather than fake filesystem paths. AetherAI must never render an imported chat message as though it were a local source file.

All injected retrieval context remains bounded by Task 11's live-model token limit.

## Memory policy

Imported ChatGPT prose does **not** automatically become durable authoritative project memory.

Memory may be created only through:

1. explicit user approval;
2. deterministic existing rules such as recognized verification artifacts;
3. later separately approved deterministic import rules.

Task 10 verification-PASS logic remains strict: a chat message containing `...=PASS` is not itself a build checkpoint. Only eligible verification artifacts parsed by the existing strict verification parser can create deterministic PASS checkpoint memory.

## Verification files and attachments

When the export contains actual attachment bytes, AetherAI imports them through the existing attachment registry and permission/index pipeline.

Recognized `*-VERIFY.txt` and `*-PACKAGE-VERIFY*.txt` files flow through Task 10's parser and may create BuildCheckpoint/CurrentBaseline memory.

If the export only references a missing attachment, AetherAI preserves the relationship as unavailable rather than fabricating file content.

## Project reconstruction

Explicit external project IDs/names are authoritative when present.

When only conversation history is present, AetherAI preserves the conversation and its provenance without guessing a project. A UI grouping such as `ChatGPT Imports` may display those records, but that grouping is not persisted as a fabricated external project identity.

## UI flow

`Settings / Data & Integrations / ChatGPT`

- `Connect ChatGPT` — opens official OpenAI/ChatGPT login/export surface in the system browser.
- `Import ChatGPT Export` — choose the downloaded official export.
- `Import latest export` — optional convenience action limited to an explicitly approved Downloads location.
- Import progress shows projects, chats, messages, files, skipped/unsupported records, and errors.
- Re-import shows added/updated/unchanged counts.
- Every imported project/chat exposes source provenance and export fingerprint.

No separate Work mode or ChatGPT-only conversation runtime is created.

## Failure and safety behavior

Unsupported or changed export schemas produce an explicit diagnostic and preserve the source archive unchanged.

A partially failed import uses a transaction/checkpoint strategy so already committed normalized records are internally consistent. It never deletes local project files or local chat history.

Import cancellation stops future parsing/writes and leaves already committed batches valid and auditable.

No importer failure may fabricate assistant text.

## Platform and packaging constraints

Task 11A remains part of AetherAI v0.3.0 until canonical publication.

Linux x86_64 and Windows x86_64 remain separate release contracts.

No new crates.io dependency may be introduced unless it is already present in the verified vendor tree and passes the self-contained packaging gate. ZIP support must be verified during Task 11A preflight before implementation.

## Acceptance endpoints

The Task 11A host verifier will eventually require:

```text
AETHERAI_CHATGPT_IMPORT_AUTH_BOUNDARY=PASS
AETHERAI_CHATGPT_IMPORT_ALL=PASS
AETHERAI_CHATGPT_IMPORT_IDEMPOTENT=PASS
AETHERAI_CHATGPT_IMPORT_PROVENANCE=PASS
AETHERAI_CHATGPT_IMPORT_PROJECT_RECONSTRUCTION=PASS
AETHERAI_CHATGPT_IMPORT_NO_FAKE_PROJECTS=PASS
AETHERAI_CHATGPT_IMPORT_ATTACHMENT_BRIDGE=PASS
AETHERAI_CHATGPT_IMPORT_PASS_CHECKPOINT_BRIDGE=PASS
AETHERAI_CHATGPT_IMPORT_RETRIEVAL=PASS
AETHERAI_CHATGPT_IMPORT_MEMORY_SAFETY=PASS
AETHERAI_CHATGPT_IMPORT_NO_CREDENTIAL_STORAGE=PASS
AETHERAI_V0_3_0_TASK11A=PASS
```

## Native archive extraction amendment

The v0.3.0 host preflight proved that no ZIP reader crate is present in the locked vendor tree. Task 11A therefore uses the existing `aether-system` platform boundary for archive extraction instead of introducing a network dependency.

Linux prefers `bsdtar` and falls back to `unzip`. Windows uses PowerShell/.NET ZIP support. Before extraction, every entry is listed and validated: absolute paths, parent traversal, unsupported names, and link/device entries are rejected. The extracted tree is re-validated to remain inside the destination and to contain no symbolic links.

This amendment preserves the no-new-non-vendored-dependency rule and does not change the official-export-only authentication boundary.

# OpenSanctuary v0.1.0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a native Linux Rust launcher and diagnostic engine foundation that detects a user-owned Diablo III installation, probes its content metadata, reports native runtime readiness, and presents a Battle.net-inspired OpenSanctuary UI without Wine/Proton or Blizzard assets.

**Architecture:** A Rust workspace separates shared state, install discovery, content probing, diagnostics, engine runtime, and launcher presentation into focused crates. The launcher is an eframe/egui native desktop application; the engine is a separate native ELF process using the same native graphics stack and a short-lived JSON launch descriptor. CASC support is read-only and begins with independently understood installation/build metadata and index discovery rather than undocumented mutation.

**Tech Stack:** Rust 2024 edition (MSRV 1.92 for eframe 0.35), serde/serde_json/toml, thiserror, eframe/egui with wgpu, tempfile, Arch Linux PKGBUILD.

## Global Constraints

- Native Linux ELF binaries only.
- No Wine, Proton, DXVK, VKD3D, Windows VM, or Windows runtime libraries.
- No Blizzard source code, executables, logos, artwork, fonts, cinematics, audio, or redistributed proprietary assets.
- No Battle.net password collection or imitation login form.
- User game data is read-only in v0.1.0.
- Minimum launcher target size: 1100x700.
- Primary development target: Arch Linux.
- Full Diablo III gameplay and official Battle.net multiplayer are non-goals for v0.1.0.
- Automated tests must run without Blizzard proprietary files.

---

## File Map

- `Cargo.toml` — workspace members and shared dependency versions.
- `crates/sanctuary-core/src/lib.rs` — shared install/runtime/config/task types and typed errors.
- `crates/sanctuary-install/src/lib.rs` — install location validation and metadata discovery.
- `crates/sanctuary-casc/src/lib.rs` — read-only content store probe/index summary.
- `crates/sanctuary-assets/src/lib.rs` — engine-facing metadata asset handles.
- `crates/sanctuary-render/src/lib.rs` — native Vulkan readiness boundary.
- `crates/sanctuary-audio/src/lib.rs` — native PipeWire readiness boundary.
- `crates/sanctuary-input/src/lib.rs` — platform-neutral input action model.
- `crates/sanctuary-diagnostics/src/lib.rs` — native system/install diagnostics and JSON/text export.
- `crates/sanctuary-engine/src/lib.rs` — launch descriptor validation and native diagnostic runtime.
- `crates/sanctuary-launcher/src/lib.rs` — launcher state machine/orchestration, independent from drawing.
- `apps/launcher/src/main.rs` — Battle.net-inspired egui shell and native launcher entry point.
- `apps/game/src/main.rs` — native diagnostic engine entry point.
- `packaging/arch/PKGBUILD` — Arch package definition.
- `packaging/arch/opensanctuary.desktop` — desktop entry.
- `assets/opensanctuary/` — original vector/text assets only.

---

### Task 1: Workspace, Shared Types, and XDG Configuration

**Files:**
- Create: `Cargo.toml`
- Create: `crates/sanctuary-core/Cargo.toml`
- Create: `crates/sanctuary-core/src/lib.rs`

**Interfaces:**
- Produces: `InstallState`, `InstallHealth`, `RuntimeCapabilities`, `AppConfig`, `LaunchDescriptor`, `SanctuaryError`.
- Produces: `AppConfig::load_from(path: &Path) -> Result<Self, SanctuaryError>` and `save_to(&self, path: &Path) -> Result<(), SanctuaryError>`.
- Produces: `LaunchDescriptor::validate(&self) -> Result<(), SanctuaryError>`.

- [ ] **Step 1: Create workspace manifest and failing config/descriptor tests**

```rust
#[test]
fn config_round_trips() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let cfg = AppConfig { reduced_motion: true, ..Default::default() };
    cfg.save_to(&path).unwrap();
    assert_eq!(AppConfig::load_from(&path).unwrap(), cfg);
}

#[test]
fn descriptor_rejects_nonexistent_install() {
    let d = LaunchDescriptor::new(PathBuf::from("/definitely/missing"));
    assert!(matches!(d.validate(), Err(SanctuaryError::InstallationNotFound(_))));
}
```

- [ ] **Step 2: Run `cargo test -p sanctuary-core` and verify failure before implementation.**

Expected: compilation failure because the shared types/functions are not defined.

- [ ] **Step 3: Implement typed state/config/descriptor model**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InstallState { NotConfigured, Searching, FoundUnindexed, Indexing, Ready, NeedsRepair, UnsupportedBuild, Error }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub install_path: Option<PathBuf>,
    pub reduced_motion: bool,
    pub selected_game: String,
}

impl LaunchDescriptor {
    pub fn validate(&self) -> Result<(), SanctuaryError> {
        if !self.install_path.is_dir() {
            return Err(SanctuaryError::InstallationNotFound(self.install_path.clone()));
        }
        Ok(())
    }
}
```

Config IO must create parent directories, use TOML, and never contain credentials.

- [ ] **Step 4: Run `cargo test -p sanctuary-core` and verify all tests pass.**

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/sanctuary-core
git commit -m "feat: establish OpenSanctuary core workspace"
```

### Task 2: Diablo III Installation Discovery

**Files:**
- Create: `crates/sanctuary-install/Cargo.toml`
- Create: `crates/sanctuary-install/src/lib.rs`

**Interfaces:**
- Consumes: `sanctuary_core::{InstallHealth, InstallState, SanctuaryError}`.
- Produces: `InstallProbe`, `probe_install(path: &Path) -> Result<InstallProbe, SanctuaryError>`.
- Produces: `discover_candidates(home: &Path) -> Vec<PathBuf>`.

- [ ] **Step 1: Write synthetic install tests**

```rust
#[test]
fn accepts_install_with_build_info_and_data_dir() {
    let td = tempfile::tempdir().unwrap();
    std::fs::write(td.path().join(".build.info"), "Branch!STRING:0|Build Key!HEX:16\nD3|00112233445566778899aabbccddeeff\n").unwrap();
    std::fs::create_dir(td.path().join("Data")).unwrap();
    let p = probe_install(td.path()).unwrap();
    assert_eq!(p.state, InstallState::FoundUnindexed);
}

#[test]
fn never_requires_diablo_exe() {
    let td = fixture_install();
    assert!(probe_install(td.path()).is_ok());
    assert!(!td.path().join("Diablo III.exe").exists());
}
```

- [ ] **Step 2: Run `cargo test -p sanctuary-install`; expect failure.**

- [ ] **Step 3: Implement read-only install validation**

`probe_install` must require `.build.info` plus a `Data` directory, capture the build-info text, and report actionable `InstallationNotFound` or `ContentStoreUnreadable` errors. `discover_candidates` may check user-home locations such as `Games/Diablo III`, `.local/share/opensanctuary/imports/Diablo III`, and mounted-library-like paths but must not execute anything.

- [ ] **Step 4: Run tests and verify passing.**

- [ ] **Step 5: Commit**

```bash
git add crates/sanctuary-install
git commit -m "feat: discover Diablo III installations natively"
```

### Task 3: Read-Only CASC/Content Metadata Probe

**Files:**
- Create: `crates/sanctuary-casc/Cargo.toml`
- Create: `crates/sanctuary-casc/src/lib.rs`

**Interfaces:**
- Consumes: installation paths from `sanctuary-install`.
- Produces: `ContentStoreSummary { build_key, data_files, index_files, total_bytes }`.
- Produces: `probe_content_store(path: &Path) -> Result<ContentStoreSummary, SanctuaryError>`.

- [ ] **Step 1: Write generated-fixture tests**

```rust
#[test]
fn enumerates_synthetic_idx_files() {
    let td = fixture_content_store();
    let summary = probe_content_store(td.path()).unwrap();
    assert_eq!(summary.index_files, 2);
    assert!(summary.total_bytes > 0);
}

#[test]
fn corrupt_or_missing_data_is_reported_without_mutation() {
    let td = tempfile::tempdir().unwrap();
    let err = probe_content_store(td.path()).unwrap_err();
    assert!(matches!(err, SanctuaryError::ContentStoreUnreadable(_)));
}
```

- [ ] **Step 2: Run `cargo test -p sanctuary-casc`; expect failure.**

- [ ] **Step 3: Implement metadata parser and read-only enumeration**

Parse `.build.info` headers defensively, extract the first available build/content key, then walk `Data/` for `.idx` and data/archive files. Do not claim full file decoding in v0.1.0 and never write into the game directory.

- [ ] **Step 4: Run tests and verify passing.**

- [ ] **Step 5: Commit**

```bash
git add crates/sanctuary-casc
git commit -m "feat: add read-only content metadata probe"
```

### Task 4: Native Diagnostics

**Files:**
- Create: `crates/sanctuary-diagnostics/Cargo.toml`
- Create: `crates/sanctuary-diagnostics/src/lib.rs`

**Interfaces:**
- Produces: `DiagnosticsReport` with OS/kernel, GPU hint, Vulkan loader availability, PipeWire availability, install summary, OpenSanctuary version, and trace ID.
- Produces: `collect(install: Option<&Path>) -> DiagnosticsReport`, `to_text()`, `to_json()`.

- [ ] **Step 1: Write serialization and capability tests**

```rust
#[test]
fn report_exports_json_without_credentials() {
    let report = DiagnosticsReport::minimal_for_test();
    let json = report.to_json().unwrap();
    assert!(json.contains("opensanctuary_version"));
    assert!(!json.to_ascii_lowercase().contains("password"));
}
```

- [ ] **Step 2: Run `cargo test -p sanctuary-diagnostics`; expect failure.**

- [ ] **Step 3: Implement native capability collection**

Vulkan readiness should check for a loader/library or `vulkaninfo` presence without invoking Windows components. PipeWire readiness should check native runtime/socket/process/library indicators. All checks are advisory and must not panic when unavailable.

- [ ] **Step 4: Run tests and verify passing.**

- [ ] **Step 5: Commit**

```bash
git add crates/sanctuary-diagnostics
git commit -m "feat: report native Linux runtime diagnostics"
```

### Task 5: Native Engine Diagnostic Runtime

**Files:**
- Create: `crates/sanctuary-engine/Cargo.toml`
- Create: `crates/sanctuary-engine/src/lib.rs`
- Create: `apps/game/Cargo.toml`
- Create: `apps/game/src/main.rs`

**Interfaces:**
- Consumes: `LaunchDescriptor` JSON path passed as `--descriptor <path>`.
- Produces: `run_descriptor(path: &Path) -> Result<(), SanctuaryError>`.
- Produces binary: `opensanctuary-engine`.

- [ ] **Step 1: Write descriptor loading tests including invalid/missing path.**

```rust
#[test]
fn loads_valid_descriptor() {
    let td = fixture_install();
    let descriptor = LaunchDescriptor::new(td.path().to_path_buf());
    let path = td.path().join("launch.json");
    std::fs::write(&path, serde_json::to_vec(&descriptor).unwrap()).unwrap();
    assert!(load_descriptor(&path).unwrap().validate().is_ok());
}
```

- [ ] **Step 2: Run `cargo test -p sanctuary-engine`; expect failure.**

- [ ] **Step 3: Implement descriptor loader and native diagnostic window**

The binary must use eframe with the wgpu renderer and display an original OpenSanctuary diagnostic scene: animated subtle star/ember field, installation path, native-renderer status, and Escape/Close exit. It must not load or execute `Diablo III.exe`.

- [ ] **Step 4: Run unit tests, then `cargo build -p opensanctuary-engine`.**

Expected: native Linux executable at `target/debug/opensanctuary-engine`.

- [ ] **Step 5: Commit**

```bash
git add crates/sanctuary-engine apps/game
git commit -m "feat: add native OpenSanctuary engine bootstrap"
```

### Task 6: Launcher State Machine and Battle.net-Inspired Native UI

**Files:**
- Create: `crates/sanctuary-launcher/Cargo.toml`
- Create: `crates/sanctuary-launcher/src/lib.rs`
- Create: `apps/launcher/Cargo.toml`
- Create: `apps/launcher/src/main.rs`
- Create: `assets/opensanctuary/README.md`

**Interfaces:**
- Consumes: install probe, content summary, diagnostics, config.
- Produces: `LauncherModel`, `PrimaryAction::{Locate, Index, Play, Repair}`, state transition methods.
- Produces binary: `opensanctuary-launcher`.

- [ ] **Step 1: Write state-driven action tests**

```rust
#[test]
fn ready_state_exposes_play() {
    let model = LauncherModel::for_state(InstallState::Ready);
    assert_eq!(model.primary_action(), PrimaryAction::Play);
}

#[test]
fn missing_install_exposes_locate() {
    let model = LauncherModel::for_state(InstallState::NotConfigured);
    assert_eq!(model.primary_action(), PrimaryAction::Locate);
}
```

- [ ] **Step 2: Run `cargo test -p sanctuary-launcher`; expect failure.**

- [ ] **Step 3: Implement launcher model and asynchronous-safe task events**

The model owns UI-facing state only. Probe/index work reports `TaskEvent::{Started,Progress,Finished,Failed}` through a channel so egui repainting never blocks on filesystem work.

- [ ] **Step 4: Implement Battle.net-inspired shell in egui**

Required layout:

```text
┌──────┬────────────────────────────────────────────────────────────┐
│ OS   │ OpenSanctuary   GAMES   ACTIVITY               ● Native  │
│ rail ├────────────────────────────────────────────────────────────┤
│ D3   │                                                            │
│      │              ORIGINAL SANCTUARY HERO FIELD                 │
│      │                                                            │
│      │  DIABLO III                                                │
│      │  Native compatibility project                              │
│      │                                                            │
│      │  [ INSTALL / LOCATE / PLAY ]   Version / readiness         │
│      ├────────────────────────────────────────────────────────────┤
│ ⚙    │ ▾ Downloads & Activity                                    │
└──────┴────────────────────────────────────────────────────────────┘
```

Use original typography via system fonts, graphite panels, cool-blue accents, restrained amber state accents, 6-10 px rounded cards, 150-250 ms fades unless `reduced_motion`, and a procedurally drawn hero treatment—no Blizzard art or logos. Minimum window size is 1100x700.

- [ ] **Step 5: Run `cargo test -p sanctuary-launcher` and `cargo build -p opensanctuary-launcher`.**

- [ ] **Step 6: Commit**

```bash
git add crates/sanctuary-launcher apps/launcher assets/opensanctuary
git commit -m "feat: build Battle.net-inspired native launcher shell"
```

### Task 7: Launcher to Engine Lifecycle and Diagnostics Export

**Files:**
- Modify: `crates/sanctuary-launcher/src/lib.rs`
- Modify: `apps/launcher/src/main.rs`
- Test: `crates/sanctuary-launcher/src/lib.rs`

**Interfaces:**
- Produces: `write_launch_descriptor(cache_dir, install_path) -> Result<PathBuf, SanctuaryError>`.
- Produces: `spawn_engine(engine_path, descriptor_path) -> Result<Child, SanctuaryError>`.
- Produces: diagnostics export action to a user-selected/native state path.

- [ ] **Step 1: Write lifecycle tests with a test child command.**

```rust
#[test]
fn descriptor_is_short_lived_and_valid() {
    let td = fixture_install();
    let cache = tempfile::tempdir().unwrap();
    let path = write_launch_descriptor(cache.path(), td.path()).unwrap();
    let descriptor: LaunchDescriptor = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert!(descriptor.validate().is_ok());
}
```

- [ ] **Step 2: Run launcher tests; verify failure.**

- [ ] **Step 3: Implement native child launch and exit tracking.**

Only enable Play when install/content/runtime readiness is `Ready`. Spawn `opensanctuary-engine --descriptor <json>`, store its PID/exit state, and surface crashes in the activity tray. Never substitute a Windows executable.

- [ ] **Step 4: Implement text/JSON diagnostics export from launcher settings.**

- [ ] **Step 5: Run `cargo test --workspace` and a debug build of both binaries.**

- [ ] **Step 6: Commit**

```bash
git add crates/sanctuary-launcher apps/launcher
git commit -m "feat: connect launcher to native engine runtime"
```

### Task 8: Arch Packaging and Release Verification

**Files:**
- Create: `packaging/arch/PKGBUILD`
- Create: `packaging/arch/opensanctuary.desktop`
- Create: `packaging/arch/opensanctuary.svg`
- Create: `README.md`

**Interfaces:**
- Produces packages/binaries: `opensanctuary-launcher`, `opensanctuary-engine`.

- [ ] **Step 1: Add package metadata and native-only dependency assertions.**

`PKGBUILD` must depend on native Linux libraries only and must not mention wine, proton, dxvk, vkd3d, winetricks, bottles, or lutris as runtime dependencies.

- [ ] **Step 2: Add README build/run instructions**

```bash
cargo build --release --workspace
./target/release/opensanctuary-launcher
```

Document that users select their own legally obtained Diablo III installation and that v0.1.0 is an engine/launcher foundation, not a complete gameplay replacement.

- [ ] **Step 3: Verify workspace**

Run:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --release -p opensanctuary-launcher -p opensanctuary-engine
file target/release/opensanctuary-launcher target/release/opensanctuary-engine
! grep -Eir '(^|[^a-z])(wine|proton|dxvk|vkd3d)([^a-z]|$)' Cargo.toml crates apps packaging/arch/PKGBUILD
```

Expected: formatting/clippy/tests/build all pass; both binaries report ELF; forbidden compatibility-layer dependency grep returns no matches.

- [ ] **Step 4: Commit**

```bash
git add packaging README.md
git commit -m "packaging: add Arch Linux OpenSanctuary package"
```

- [ ] **Step 5: Tag-ready status check**

```bash
git status --short
git log --oneline --decorate -8
```

Expected: clean feature branch with eight focused implementation commits after the design/plan commits.

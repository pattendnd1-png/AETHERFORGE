# Darkstone-RS Initial Rebuild Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the first Darkstone-RS offline compatibility foundation: a compiling Rust workspace that safely reads user-owned Darkstone MTF archives, decodes static O3D geometry into neutral meshes, renders the same mesh in Classic and basic Reforged modes, and emits a machine-readable compatibility report without bundling original assets.

**Architecture:** Binary compatibility ends in dedicated `darkstone-mtf` and `darkstone-o3d` crates. Both produce neutral data owned by `darkstone-assets`; `darkstone-render` consumes only neutral meshes and exposes Classic/Reforged presentation policies over one wgpu renderer. `darkstone-compat-report` scans a read-only user-selected install and reports archive/asset support; the `darkstone` app composes scanner → archive → O3D → renderer for the first vertical slice.

**Tech Stack:** Rust stable; edition 2024; `wgpu` 30.0.1; `winit` 0.30.13; `glam` 0.33; `bytemuck` 1.25; `serde` 1.0; `serde_json` 1.0; `sha2` 0.10; `thiserror` 2; `tracing` 0.1; `tracing-subscriber` 0.3; `clap` 4; `pollster` 0.4.

**Spec:** `docs/superpowers/specs/2026-08-25-darkstone-rs-design.md`

## Global Constraints

- Language: Rust stable.
- GPU API abstraction: wgpu.
- Shader language: WGSL.
- Window/event layer: winit unless a concrete blocker appears.
- Math: glam or equivalent lightweight Rust math crate after dependency review.
- Serialization: explicit versioned formats; serde may be used for tooling/configuration, not blindly for binary compatibility formats.
- Logging/diagnostics: tracing-style structured logging.
- Asset hashing: SHA-256 for compatibility reports.
- Original copyrighted Darkstone data is user-supplied and never committed or packaged.
- Offline campaign compatibility through Milestone 5 is a hard gate before online feature implementation.
- No networking, persistence, realm, zone, guild, chat, or MMO work is part of this plan.
- Parsers use checked arithmetic, bounds checks, explicit allocation limits, and structured errors.
- Classic and Reforged are policies over one canonical renderer/mesh representation, never separate engines.

---

## File Structure

```text
Cargo.toml                         # workspace members, shared dependency versions/lints
rust-toolchain.toml                # stable channel + rustfmt/clippy components
.gitignore                         # build/cache/local original-data exclusions
LICENSES/README.md                 # original-data non-redistribution boundary
crates/darkstone-assets/           # neutral mesh/material/installation/report types
crates/darkstone-mtf/              # MTF directory parser + decompressor
crates/darkstone-o3d/              # O3D static-geometry decoder
crates/darkstone-render/           # wgpu device setup + Classic/Reforged mesh rendering
apps/darkstone/                    # winit viewer vertical slice
tools/darkstone-compat-report/     # read-only install scanner + JSON report CLI
shaders/classic.wgsl               # simple compatibility shading
shaders/reforged.wgsl              # first PBR-ish lit material path
tests/fixtures/                    # synthetic, redistributable MTF/O3D fixtures only
```

### Task 1: Create the canonical Rust workspace and neutral asset contracts

**Files:**
- Create: `Cargo.toml`
- Create: `rust-toolchain.toml`
- Create: `.gitignore`
- Create: `LICENSES/README.md`
- Create: `crates/darkstone-assets/Cargo.toml`
- Create: `crates/darkstone-assets/src/lib.rs`
- Test: `crates/darkstone-assets/src/lib.rs` unit tests

**Interfaces:**
- Consumes: nothing.
- Produces: `Mesh`, `MeshVertex`, `MeshTriangle`, `MaterialSlot`, `Aabb`, `InstallCandidate`, `CompatibilityReport`, and `AssetStatus` used by every later task.

- [ ] **Step 1: Write the neutral mesh unit tests**

Add tests proving AABB computation and triangle index validation:

```rust
#[test]
fn aabb_covers_all_vertices() {
    let mesh = Mesh::new(
        vec![
            MeshVertex::new([-2.0, 1.0, 4.0]),
            MeshVertex::new([3.0, -5.0, 1.0]),
            MeshVertex::new([0.0, 2.0, 9.0]),
        ],
        vec![MeshTriangle::new([0, 1, 2], 0)],
        vec![MaterialSlot::new(15)],
    ).unwrap();
    assert_eq!(mesh.bounds.min, [-2.0, -5.0, 1.0]);
    assert_eq!(mesh.bounds.max, [3.0, 2.0, 9.0]);
}

#[test]
fn rejects_out_of_range_triangle_index() {
    let err = Mesh::new(
        vec![MeshVertex::new([0.0, 0.0, 0.0])],
        vec![MeshTriangle::new([0, 1, 0], 0)],
        vec![MaterialSlot::new(0)],
    ).unwrap_err();
    assert!(matches!(err, MeshError::VertexIndexOutOfRange { index: 1, .. }));
}
```

- [ ] **Step 2: Run the test and verify it fails**

Run: `cargo test -p darkstone-assets`

Expected: FAIL because the workspace/crate/types do not exist.

- [ ] **Step 3: Create the workspace manifests and neutral types**

Root workspace:

```toml
[workspace]
resolver = "3"
members = [
  "crates/darkstone-assets",
]
# Add later crates to `members` in the task that creates them so every intermediate workspace compiles.

[workspace.package]
edition = "2024"
license = "MIT OR Apache-2.0"
rust-version = "1.87"
version = "0.1.0"

[workspace.dependencies]
bytemuck = { version = "1.25", features = ["derive"] }
clap = { version = "4", features = ["derive"] }
glam = "0.33"
pollster = "0.4"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
sha2 = "0.10"
thiserror = "2"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt"] }
wgpu = "30.0.1"
winit = "0.30.13"
```

`darkstone-assets` exposes:

```rust
pub struct MeshVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub color: [u8; 4],
}

pub struct MeshTriangle {
    pub indices: [u32; 3],
    pub material_slot: u16,
}

pub struct MaterialSlot {
    pub source_number: u16,
}

pub struct Mesh {
    pub vertices: Vec<MeshVertex>,
    pub triangles: Vec<MeshTriangle>,
    pub materials: Vec<MaterialSlot>,
    pub bounds: Aabb,
}
```

`Mesh::new` validates every triangle index/material slot and computes bounds. `CompatibilityReport` is `Serialize` and contains `schema_version: 1`, install path, archive hashes, per-entry status, and summary counts.

- [ ] **Step 4: Run formatting and tests**

Run: `cargo fmt --all --check && cargo test -p darkstone-assets`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml rust-toolchain.toml .gitignore LICENSES crates/darkstone-assets
git commit -m "build: create Darkstone-RS Rust workspace"
```

### Task 2: Implement hardened MTF directory parsing and custom decompression

**Files:**
- Create: `crates/darkstone-mtf/Cargo.toml`
- Create: `crates/darkstone-mtf/src/lib.rs`
- Create: `crates/darkstone-mtf/src/reader.rs`
- Create: `crates/darkstone-mtf/src/decompress.rs`
- Create: `tests/fixtures/mtf/make_fixture.py`
- Test: `crates/darkstone-mtf/tests/archive.rs`

**Interfaces:**
- Consumes: byte slices or read-only archive files.
- Produces: `MtfArchive::parse(Arc<[u8]>, Limits) -> Result<MtfArchive, MtfError>`, `MtfArchive::entries() -> &[MtfEntry]`, and `MtfArchive::read_entry(index) -> Result<Vec<u8>, MtfError>`.

- [ ] **Step 1: Write synthetic archive tests**

Build redistributable in-memory fixtures with this known layout: little-endian `u32` entry count, then for each entry `u32 filename_len`, filename bytes including NUL, absolute `u32 data_offset`, and `u32 uncompressed_size`. Test one raw entry and one compressed entry. The compressed fixture encodes eight literal bytes with flag byte `0xFF`, so expected output is deterministic without relying on proprietary assets.

```rust
#[test]
fn parses_one_uncompressed_entry() {
    let bytes = fixture::single_raw("TEST\\HELLO.TXT\0", b"hello");
    let archive = MtfArchive::parse(bytes.into(), Limits::default()).unwrap();
    assert_eq!(archive.entries()[0].name, "TEST\\HELLO.TXT");
    assert_eq!(archive.read_entry(0).unwrap(), b"hello");
}

#[test]
fn decompresses_literal_chunk() {
    let bytes = fixture::single_compressed("A.BIN\0", b"12345678");
    let archive = MtfArchive::parse(bytes.into(), Limits::default()).unwrap();
    assert_eq!(archive.read_entry(0).unwrap(), b"12345678");
}
```

Also test malformed filename lengths, decreasing offsets, out-of-file offsets, compression headers whose decompressed size disagrees with the directory, zero back-reference offsets, back-references before output start, and decompression past the advertised size.

- [ ] **Step 2: Run the tests and verify they fail**

Run: `cargo test -p darkstone-mtf`

Expected: FAIL because parser/decompressor are absent.

- [ ] **Step 3: Implement safe little-endian cursor helpers and directory parsing**

Use checked helpers rather than transmuting structs:

```rust
fn take_u32(bytes: &[u8], at: &mut usize) -> Result<u32, MtfError> {
    let end = at.checked_add(4).ok_or(MtfError::ArithmeticOverflow)?;
    let raw = bytes.get(*at..end).ok_or(MtfError::UnexpectedEof { offset: *at })?;
    *at = end;
    Ok(u32::from_le_bytes(raw.try_into().unwrap()))
}
```

Limits:

```rust
pub struct Limits {
    pub max_entries: u32,        // default 100_000
    pub max_name_bytes: u32,     // default 1024
    pub max_entry_output: u64,   // default 512 MiB
}
```

Normalize only the terminal NUL for the logical entry name; preserve the original archive path string otherwise. Compute each stored span from the next greater data offset or EOF without sorting the canonical directory order.

- [ ] **Step 4: Implement decompression with strict bounds**

Compression header is exactly 12 bytes: `magic1`, `magic2`, unknown `u16`, compressed size `u32`, decompressed size `u32`. Treat `(0xAE|0xAF, 0xBE)` as compressed. For each control byte, process bits low-to-high. Set bit means one literal byte. Clear bit means a little-endian control word where `count = (word >> 10) + 3` and `offset = word & 0x03ff`; reject `offset == 0` or `offset > output.len()`, then copy one byte at a time from `output.len() - offset` so overlapping back-references work exactly like LZ-style copies.

```rust
for _ in 0..count {
    let src = out.len().checked_sub(offset as usize)
        .ok_or(MtfError::InvalidBackReference { offset, produced: out.len() })?;
    let byte = out[src];
    out.push(byte);
    if out.len() > expected_len { return Err(MtfError::OutputOverflow); }
}
```

Stop only when exactly `expected_len` bytes have been produced. Never allocate from an unvalidated archive size.

- [ ] **Step 5: Run all parser tests**

Run: `cargo test -p darkstone-mtf`

Expected: PASS.

- [ ] **Step 6: Run parser-specific lint gate**

Run: `cargo clippy -p darkstone-mtf --all-targets -- -D warnings`

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/darkstone-mtf tests/fixtures/mtf
git commit -m "feat: add hardened Darkstone MTF reader"
```

### Task 3: Add read-only installation discovery and SHA-256 compatibility inventory

**Files:**
- Create: `tools/darkstone-compat-report/Cargo.toml`
- Create: `tools/darkstone-compat-report/src/main.rs`
- Create: `tools/darkstone-compat-report/src/scan.rs`
- Create: `tools/darkstone-compat-report/src/report.rs`
- Test: `tools/darkstone-compat-report/tests/scan.rs`

**Interfaces:**
- Consumes: explicit `--install PATH` plus optional platform discovery candidates.
- Produces: `scan_install(path: &Path) -> Result<CompatibilityReport, ScanError>` and CLI JSON to stdout or `--output PATH`.

- [ ] **Step 1: Write scanner tests around a temporary fake install**

Create a temp directory containing only synthetic `DATA.MTF`, `MUSIC.MTF`, and unrelated files. Assert that scanning does not modify any mtime/content, hashes the archives with SHA-256, enumerates entries via `darkstone-mtf`, and reports missing optional archives without failing the whole scan.

- [ ] **Step 2: Run the scanner tests and verify failure**

Run: `cargo test -p darkstone-compat-report`

Expected: FAIL because the scanner is absent.

- [ ] **Step 3: Implement explicit-path scanning first**

The canonical first path is `--install`; discovery helpers may suggest common locations but must never choose silently. Candidate validation requires at least `DATA.MTF`. Open every file read-only, hash with streaming SHA-256, parse MTF metadata, and record errors per archive/entry rather than panicking.

CLI contract:

```text
darkstone-compat-report --install /path/to/Darkstone --output report.json
```

Exit 0 when the install is valid even if unsupported asset formats remain; exit 2 for invalid CLI/path; exit 3 when `DATA.MTF` cannot be parsed safely.

- [ ] **Step 4: Run scanner tests and manual synthetic report**

Run:

```bash
cargo test -p darkstone-compat-report
cargo run -p darkstone-compat-report -- --install tests/fixtures/fake-install
```

Expected: tests PASS and stdout is schema-version-1 JSON.

- [ ] **Step 5: Commit**

```bash
git add tools/darkstone-compat-report
git commit -m "feat: add read-only Darkstone install scanner"
```

### Task 4: Decode static O3D geometry into neutral meshes

**Files:**
- Create: `crates/darkstone-o3d/Cargo.toml`
- Create: `crates/darkstone-o3d/src/lib.rs`
- Create: `crates/darkstone-o3d/src/parse.rs`
- Create: `tests/fixtures/o3d/make_fixture.py`
- Test: `crates/darkstone-o3d/tests/static_mesh.rs`

**Interfaces:**
- Consumes: decompressed O3D bytes.
- Produces: `decode_o3d(bytes: &[u8], limits: O3dLimits) -> Result<Mesh, O3dError>`.

- [ ] **Step 1: Write a synthetic O3D test with one triangle and one quad**

Known static O3D layout is little-endian: `vertex_count u32`, `face_count u32`, two unknown `u32`, `vertex_count` positions of three `f32` values, then `face_count` fixed 50-byte faces. Each face is BGRA `u8[4]`, four UV pairs (`8 x f32`), four `u16` vertex indices, unknown `u32`, and material/texture number `u16`. Index 3 equal to `0xffff` marks a triangle.

Assert that the quad is triangulated deterministically as `(0,1,2)` and `(0,2,3)`, UVs are normalized by `1/256`, BGRA becomes RGBA, material number is retained, and bounds match positions.

- [ ] **Step 2: Run the O3D tests and verify failure**

Run: `cargo test -p darkstone-o3d`

Expected: FAIL because decoder is absent.

- [ ] **Step 3: Implement bounds-checked O3D parsing**

Validate counts before multiplication/allocation:

```rust
let vertex_bytes = (vertex_count as usize)
    .checked_mul(12)
    .ok_or(O3dError::ArithmeticOverflow)?;
let face_bytes = (face_count as usize)
    .checked_mul(50)
    .ok_or(O3dError::ArithmeticOverflow)?;
```

Defaults: max 5,000,000 vertices, max 5,000,000 faces, max 512 MiB input. Reject NaN/infinite positions and every index outside the decoded vertex array. Build neutral vertices per face corner when UV/color discontinuities require it; do not leak O3D face layout into `darkstone-render`.

- [ ] **Step 4: Generate normals in neutral space**

Accumulate triangle cross products into per-vertex normal sums and normalize finite non-zero vectors. Degenerate triangles receive `[0, 1, 0]` only as a rendering fallback and are reported in decoder diagnostics.

- [ ] **Step 5: Run tests and clippy**

Run:

```bash
cargo test -p darkstone-o3d
cargo clippy -p darkstone-o3d --all-targets -- -D warnings
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/darkstone-o3d tests/fixtures/o3d
git commit -m "feat: decode Darkstone O3D static meshes"
```

### Task 5: Build one wgpu renderer with Classic and Reforged policies

**Files:**
- Create: `crates/darkstone-render/Cargo.toml`
- Create: `crates/darkstone-render/src/lib.rs`
- Create: `crates/darkstone-render/src/gpu.rs`
- Create: `crates/darkstone-render/src/mesh.rs`
- Create: `crates/darkstone-render/src/camera.rs`
- Create: `crates/darkstone-render/src/material.rs`
- Create: `shaders/classic.wgsl`
- Create: `shaders/reforged.wgsl`
- Test: `crates/darkstone-render/tests/shader_validation.rs`

**Interfaces:**
- Consumes: `darkstone_assets::Mesh` only.
- Produces: `Renderer::new`, `Renderer::upload_mesh`, `Renderer::render`, and `RenderMode::{Classic, Reforged}`.

- [ ] **Step 1: Add shader validation tests**

Use wgpu/naga shader-module creation inside a headless adapter/device test when available. A missing adapter is an explicit skipped environment condition, not a passing shader validation.

- [ ] **Step 2: Run renderer tests and verify failure**

Run: `cargo test -p darkstone-render`

Expected: FAIL because renderer/shaders are absent.

- [ ] **Step 3: Implement GPU vertex conversion and shared mesh buffers**

`GpuVertex` is the only GPU-facing representation:

```rust
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}
```

Classic and Reforged pipelines share the same vertex/index buffers and camera uniforms.

- [ ] **Step 4: Implement Classic shader**

Classic uses texture/vertex color with restrained Lambert-style directional lighting and no HDR-only dependency. Missing source texture falls back to a generated checker pattern owned by the renderer, never a bundled Darkstone texture.

- [ ] **Step 5: Implement first Reforged shader**

Reforged uses a linear HDR target, normal-based directional + ambient lighting, a material uniform with base color/roughness/metallic/emissive defaults, and ACES-like/filmic tone mapping in the final pass. This is intentionally the basic PBR foundation, not Milestone 3 effects.

- [ ] **Step 6: Run renderer validation**

Run:

```bash
cargo test -p darkstone-render
cargo clippy -p darkstone-render --all-targets -- -D warnings
```

Expected: PASS on a host with a supported graphics adapter; shader parsing must PASS independently.

- [ ] **Step 7: Commit**

```bash
git add crates/darkstone-render shaders
git commit -m "feat: add Classic and Reforged wgpu renderer"
```

### Task 6: Integrate a real user-owned O3D model path into the desktop viewer

**Files:**
- Create: `apps/darkstone/Cargo.toml`
- Create: `apps/darkstone/src/main.rs`
- Create: `apps/darkstone/src/app.rs`
- Create: `apps/darkstone/src/load.rs`
- Test: `apps/darkstone/tests/load_pipeline.rs`

**Interfaces:**
- Consumes: `--install PATH`, `--model ARCHIVE_PATH`, `--mode classic|reforged`.
- Produces: a window displaying a decoded model from user-owned data using either renderer policy.

- [ ] **Step 1: Write integration tests against synthetic MTF/O3D fixtures**

The load pipeline test builds a synthetic archive in memory, locates one `.O3D` entry by case-insensitive archive path, decompresses it, decodes it, and returns a neutral `Mesh` without creating a GPU device.

- [ ] **Step 2: Run integration test and verify failure**

Run: `cargo test -p darkstone`

Expected: FAIL because app pipeline is absent.

- [ ] **Step 3: Implement the CPU-side load pipeline**

```rust
pub fn load_mesh_from_install(
    install: &Path,
    archive_name: &str,
    model_path: &str,
) -> Result<Mesh, LoadError>;
```

Open `install.join(archive_name)` read-only, parse MTF, locate the entry, read/decompress it, and pass the resulting bytes to `decode_o3d`.

- [ ] **Step 4: Implement winit 0.30 application lifecycle**

Create the window in `ApplicationHandler::resumed`, initialize the renderer against that window, handle resize/close/keyboard input, and request redraw continuously. `C` selects Classic and `R` selects Reforged at runtime without reloading mesh data.

- [ ] **Step 5: Add orbit camera controls**

Mouse drag changes yaw/pitch; wheel changes distance; camera matrices are independent of simulation timing. Clamp pitch and distance to finite bounds.

- [ ] **Step 6: Run tests and launch with a real install when available**

Run:

```bash
cargo test -p darkstone
cargo run -p darkstone -- --install "$DARKSTONE_HOME" --archive DATA.MTF --model '...\\MODEL.O3D' --mode classic
```

Expected: synthetic tests PASS. On a host with a user-owned Darkstone install, one original model renders in Classic; pressing `R` renders the identical neutral mesh through Reforged.

- [ ] **Step 7: Commit**

```bash
git add apps/darkstone
git commit -m "feat: render user-owned Darkstone models"
```

### Task 7: Expand the compatibility report with O3D support status and copyright guardrails

**Files:**
- Modify: `tools/darkstone-compat-report/src/scan.rs`
- Modify: `tools/darkstone-compat-report/src/report.rs`
- Modify: `LICENSES/README.md`
- Create: `scripts/check-no-original-assets.sh`
- Test: `tools/darkstone-compat-report/tests/report.rs`

**Interfaces:**
- Consumes: all prior parser/neutral-model APIs.
- Produces: final Milestone-0 schema-v1 report and release guard script.

- [ ] **Step 1: Write report summary tests**

For synthetic fixtures assert:

```json
{
  "schema_version": 1,
  "summary": {
    "archives_parsed": 1,
    "entries_total": 2,
    "o3d_decoded": 1,
    "unsupported": 1,
    "errors": 0
  }
}
```

Every asset record includes archive name, internal path, uncompressed size, SHA-256 of decoded bytes, detected kind, support status, and diagnostic message when unsupported/error.

- [ ] **Step 2: Run report tests and verify failure**

Run: `cargo test -p darkstone-compat-report --test report`

Expected: FAIL until O3D classification is wired in.

- [ ] **Step 3: Implement extension/header classification and O3D decode probing**

`.O3D` entries are decoded through `darkstone-o3d`. Known but unsupported extensions are tagged `recognized-unsupported`; unknown extensions are `unknown`; parse failures are `error` with structured diagnostic context.

- [ ] **Step 4: Add original-asset repository guard**

`scripts/check-no-original-assets.sh` rejects tracked files with original-format extensions under source/test/package paths (`.MTF`, `.O3D`, `.TGA`, `.MP2`, `.WAV`, `.B3D`, `.SKA`, `.MDL`, `.CDF`, `.CLD`, `.BRM`, `.MBR`) except explicitly generated synthetic fixtures whose names start with `SYNTHETIC_`. It also rejects known archive names `DATA.MTF`, `MUSIC.MTF`, and `VOICES1.MTF` anywhere in `git ls-files`.

- [ ] **Step 5: Run the complete first-plan verification gate**

Run:

```bash
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
./scripts/check-no-original-assets.sh
cargo run -p darkstone-compat-report -- --install tests/fixtures/fake-install --output /tmp/darkstone-report.json
python3 -m json.tool /tmp/darkstone-report.json >/dev/null
```

Expected: all commands PASS.

- [ ] **Step 6: Commit**

```bash
git add tools/darkstone-compat-report LICENSES scripts
git commit -m "test: gate Darkstone Milestone 0 compatibility"
```

## Plan Self-Review

- Spec coverage: PASS for Section 31 acceptance criteria 1–9. Online/MMO requirements are deliberately excluded by the spec's hard gate.
- Placeholder scan: PASS; no deferred implementation language remains in executable tasks.
- Type consistency: PASS; MTF and O3D terminate into `darkstone-assets::Mesh`, and rendering depends only on neutral mesh types.
- Copyright boundary: PASS; all committed fixtures are synthetic and the final guard rejects original Darkstone payloads.
- Sequential gate: PASS; this plan ends at the initial offline rebuild foundation and does not implement Milestones 6–10.

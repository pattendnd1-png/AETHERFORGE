#!/usr/bin/env bash
set -euo pipefail

repo="${1:-$(pwd)}"
cd "$repo"

need() {
  command -v "$1" >/dev/null 2>&1 || {
    printf 'DARKSTONE_TASK1_BLOCKED=missing-%s\n' "$1" >&2
    exit 70
  }
}

need cargo
need rustc

in_git=false
if command -v git >/dev/null 2>&1 && git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  in_git=true
  branch="$(git branch --show-current)"
  if [[ "$branch" != "feature/initial-rebuild" ]]; then
    printf 'DARKSTONE_TASK1_BLOCKED=wrong-branch:%s\n' "$branch" >&2
    exit 71
  fi
else
  printf 'DARKSTONE_TASK1_GIT=standalone-no-commit\n'
fi

printf 'RUSTC=%s\n' "$(rustc --version)"
printf 'CARGO=%s\n' "$(cargo --version)"

set +e
red_output="$(cargo test -p darkstone-assets 2>&1)"
red_rc=$?
set -e
printf '%s\n' "$red_output"

if (( red_rc == 0 )); then
  echo 'DARKSTONE_TASK1_RED=FAIL-test-passed-before-implementation' >&2
  exit 72
fi
if ! grep -Eq 'unresolved import|unresolved imports|cannot find' <<<"$red_output"; then
  echo 'DARKSTONE_TASK1_RED=FAIL-unexpected-failure' >&2
  exit 73
fi
printf 'DARKSTONE_TASK1_RED=PASS\n'

cat > crates/darkstone-assets/src/lib.rs <<'RUST'
//! Neutral engine asset contracts for Darkstone-RS.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeshVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub color: [u8; 4],
}

impl MeshVertex {
    #[must_use]
    pub const fn new(position: [f32; 3]) -> Self {
        Self {
            position,
            normal: [0.0, 1.0, 0.0],
            uv: [0.0, 0.0],
            color: [255, 255, 255, 255],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeshTriangle {
    pub indices: [u32; 3],
    pub material_slot: u16,
}

impl MeshTriangle {
    #[must_use]
    pub const fn new(indices: [u32; 3], material_slot: u16) -> Self {
        Self {
            indices,
            material_slot,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaterialSlot {
    pub source_number: u16,
}

impl MaterialSlot {
    #[must_use]
    pub const fn new(source_number: u16) -> Self {
        Self { source_number }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

#[derive(Debug, Clone, PartialEq)]
pub struct Mesh {
    pub vertices: Vec<MeshVertex>,
    pub triangles: Vec<MeshTriangle>,
    pub materials: Vec<MaterialSlot>,
    pub bounds: Aabb,
}

impl Mesh {
    pub fn new(
        vertices: Vec<MeshVertex>,
        triangles: Vec<MeshTriangle>,
        materials: Vec<MaterialSlot>,
    ) -> Result<Self, MeshError> {
        if vertices.is_empty() {
            return Err(MeshError::EmptyVertices);
        }

        for (vertex, position) in vertices.iter().enumerate() {
            for (axis, value) in position.position.iter().copied().enumerate() {
                if !value.is_finite() {
                    return Err(MeshError::NonFinitePosition {
                        vertex,
                        axis,
                        value,
                    });
                }
            }
        }

        for (triangle, face) in triangles.iter().enumerate() {
            for &index in &face.indices {
                if index as usize >= vertices.len() {
                    return Err(MeshError::VertexIndexOutOfRange {
                        triangle,
                        index,
                        vertex_count: vertices.len(),
                    });
                }
            }
            if face.material_slot as usize >= materials.len() {
                return Err(MeshError::MaterialSlotOutOfRange {
                    triangle,
                    slot: face.material_slot,
                    material_count: materials.len(),
                });
            }
        }

        let mut min = vertices[0].position;
        let mut max = vertices[0].position;
        for vertex in &vertices[1..] {
            for axis in 0..3 {
                min[axis] = min[axis].min(vertex.position[axis]);
                max[axis] = max[axis].max(vertex.position[axis]);
            }
        }

        Ok(Self {
            vertices,
            triangles,
            materials,
            bounds: Aabb { min, max },
        })
    }
}

#[derive(Debug, Error, PartialEq)]
pub enum MeshError {
    #[error("mesh has no vertices")]
    EmptyVertices,
    #[error("vertex {vertex} axis {axis} is not finite: {value}")]
    NonFinitePosition {
        vertex: usize,
        axis: usize,
        value: f32,
    },
    #[error(
        "triangle {triangle} references vertex {index}, but the mesh has {vertex_count} vertices"
    )]
    VertexIndexOutOfRange {
        triangle: usize,
        index: u32,
        vertex_count: usize,
    },
    #[error(
        "triangle {triangle} references material slot {slot}, but the mesh has {material_count} materials"
    )]
    MaterialSlotOutOfRange {
        triangle: usize,
        slot: u16,
        material_count: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallCandidate {
    pub path: PathBuf,
    pub source: InstallSource,
}

impl InstallCandidate {
    #[must_use]
    pub fn explicit(path: PathBuf) -> Self {
        Self {
            path,
            source: InstallSource::Explicit,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InstallSource {
    Explicit,
    Steam,
    Gog,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchiveFingerprint {
    pub name: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SupportState {
    Supported,
    RecognizedUnsupported,
    Unknown,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetStatus {
    pub archive_name: String,
    pub internal_path: String,
    pub uncompressed_size: u64,
    pub sha256: Option<String>,
    pub kind: Option<String>,
    pub status: SupportState,
    pub diagnostic: Option<String>,
}

impl AssetStatus {
    #[must_use]
    pub fn new(
        archive_name: impl Into<String>,
        internal_path: impl Into<String>,
        uncompressed_size: u64,
        status: SupportState,
    ) -> Self {
        Self {
            archive_name: archive_name.into(),
            internal_path: internal_path.into(),
            uncompressed_size,
            sha256: None,
            kind: None,
            status,
            diagnostic: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilitySummary {
    pub archives_parsed: u64,
    pub entries_total: u64,
    pub o3d_decoded: u64,
    pub unsupported: u64,
    pub errors: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityReport {
    pub schema_version: u32,
    pub install_path: PathBuf,
    pub archives: Vec<ArchiveFingerprint>,
    pub assets: Vec<AssetStatus>,
    pub summary: CompatibilitySummary,
}

impl CompatibilityReport {
    #[must_use]
    pub fn new(install_path: PathBuf, assets: Vec<AssetStatus>) -> Self {
        Self {
            schema_version: 1,
            install_path,
            archives: Vec::new(),
            assets,
            summary: CompatibilitySummary::default(),
        }
    }
}
RUST

cargo fmt --all
cargo fmt --all --check
cargo test -p darkstone-assets
cargo clippy -p darkstone-assets --all-targets -- -D warnings

printf 'DARKSTONE_TASK1_GREEN=PASS\n'

if [[ "$in_git" == true ]]; then
  git add Cargo.toml rust-toolchain.toml .gitignore LICENSES crates/darkstone-assets \
    docs/superpowers/plans/2026-08-25-darkstone-rs-initial-rebuild.md
  if ! git diff --cached --quiet; then
    git commit -m 'build: create Darkstone-RS Rust workspace'
  fi
  printf 'DARKSTONE_TASK1_COMMIT=%s\n' "$(git rev-parse --short HEAD)"
else
  printf 'DARKSTONE_TASK1_COMMIT=SKIPPED-standalone\n'
fi

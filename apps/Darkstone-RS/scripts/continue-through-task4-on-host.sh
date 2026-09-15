#!/usr/bin/env bash
set -euo pipefail

repo="${1:-$(pwd)}"
cd "$repo"

need() {
  command -v "$1" >/dev/null 2>&1 || {
    printf 'DARKSTONE_TASK4_BLOCKED=missing-%s\n' "$1" >&2
    exit 70
  }
}

need cargo
need rustc
need python3

in_git=false
if command -v git >/dev/null 2>&1 && git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  in_git=true
  branch="$(git branch --show-current)"
  if [[ "$branch" != "feature/initial-rebuild" ]]; then
    printf 'DARKSTONE_TASK4_BLOCKED=wrong-branch:%s\n' "$branch" >&2
    exit 71
  fi
else
  printf 'DARKSTONE_TASK4_GIT=standalone-no-commit\n'
fi

printf 'RUSTC=%s\n' "$(rustc --version)"
printf 'CARGO=%s\n' "$(cargo --version)"

# Task 3 must be GREEN before O3D work starts.
if [[ -f tools/darkstone-compat-report/src/scan.rs ]] && grep -q '^pub fn scan_install' tools/darkstone-compat-report/src/scan.rs; then
  cargo fmt --all --check
  cargo test -p darkstone-assets
  cargo test -p darkstone-mtf
  cargo test -p darkstone-compat-report
  cargo clippy -p darkstone-compat-report --all-targets -- -D warnings
  printf 'DARKSTONE_TASK3_GREEN=PASS-existing\n'
else
  ./scripts/continue-through-task3-on-host.sh "$repo"
fi

python3 - <<'PY'
from pathlib import Path
p = Path('Cargo.toml')
s = p.read_text()
member = '  "crates/darkstone-o3d",\n'
if '"crates/darkstone-o3d"' not in s:
    marker = '  "crates/darkstone-mtf",\n'
    if marker not in s:
        raise SystemExit('DARKSTONE_TASK4_BLOCKED=workspace-members-shape')
    s = s.replace(marker, marker + member)
p.write_text(s)
PY

mkdir -p crates/darkstone-o3d/src crates/darkstone-o3d/tests
cat > crates/darkstone-o3d/Cargo.toml <<'TOML'
[package]
name = "darkstone-o3d"
version.workspace = true
edition.workspace = true
license.workspace = true
rust-version.workspace = true

[dependencies]
darkstone-assets = { path = "../darkstone-assets" }
thiserror.workspace = true
TOML

cat > crates/darkstone-o3d/src/lib.rs <<'RUST'
//! Static Darkstone O3D decoder.
//!
//! Production decoder contracts are intentionally absent in the Task-4 RED state.
RUST

cat > crates/darkstone-o3d/tests/static_mesh.rs <<'RUST'
use darkstone_o3d::{decode_o3d, O3dError, O3dLimits};

fn push_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_f32(out: &mut Vec<u8>, value: f32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn face(color_bgra: [u8; 4], uv: [[f32; 2]; 4], indices: [u16; 4], material: u16) -> Vec<u8> {
    let mut out = Vec::with_capacity(50);
    out.extend_from_slice(&color_bgra);
    for pair in uv {
        push_f32(&mut out, pair[0]);
        push_f32(&mut out, pair[1]);
    }
    for index in indices {
        push_u16(&mut out, index);
    }
    push_u32(&mut out, 0);
    push_u16(&mut out, material);
    assert_eq!(out.len(), 50);
    out
}

fn fixture(positions: &[[f32; 3]], faces: &[Vec<u8>]) -> Vec<u8> {
    let mut out = Vec::new();
    push_u32(&mut out, positions.len() as u32);
    push_u32(&mut out, faces.len() as u32);
    push_u32(&mut out, 0);
    push_u32(&mut out, 0);
    for position in positions {
        for value in position {
            push_f32(&mut out, *value);
        }
    }
    for face in faces {
        out.extend_from_slice(face);
    }
    out
}

#[test]
fn decodes_triangle_and_quad_into_neutral_mesh() {
    let positions = [
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
        [0.0, 1.0, 0.0],
    ];
    let uv = [[0.0, 0.0], [256.0, 0.0], [256.0, 256.0], [0.0, 256.0]];
    let faces = vec![
        face([10, 20, 30, 40], uv, [0, 1, 4, 0xffff], 7),
        face([1, 2, 3, 255], uv, [0, 1, 2, 3], 9),
    ];
    let mesh = decode_o3d(&fixture(&positions, &faces), O3dLimits::default()).unwrap();

    assert_eq!(mesh.vertices.len(), 7);
    assert_eq!(mesh.triangles.len(), 3);
    assert_eq!(mesh.materials.len(), 2);
    assert_eq!(mesh.materials[0].source_number, 7);
    assert_eq!(mesh.materials[1].source_number, 9);
    assert_eq!(mesh.triangles[0].indices, [0, 1, 2]);
    assert_eq!(mesh.triangles[1].indices, [3, 4, 5]);
    assert_eq!(mesh.triangles[2].indices, [3, 5, 6]);
    assert_eq!(mesh.vertices[0].color, [30, 20, 10, 40]);
    assert_eq!(mesh.vertices[1].uv, [1.0, 0.0]);
    assert_eq!(mesh.vertices[2].uv, [1.0, 1.0]);
    assert_eq!(mesh.bounds.min, [0.0, 0.0, 0.0]);
    assert_eq!(mesh.bounds.max, [1.0, 1.0, 1.0]);
    assert!(mesh.vertices.iter().all(|vertex| {
        vertex.normal.iter().all(|value| value.is_finite())
            && vertex.normal.iter().any(|value| value.abs() > 0.0)
    }));
}

#[test]
fn rejects_non_finite_positions() {
    let positions = [[f32::NAN, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
    let uv = [[0.0, 0.0]; 4];
    let bytes = fixture(&positions, &[face([0, 0, 0, 255], uv, [0, 1, 2, 0xffff], 0)]);

    let err = decode_o3d(&bytes, O3dLimits::default()).unwrap_err();
    assert!(matches!(err, O3dError::NonFinitePosition { .. }));
}

#[test]
fn rejects_face_index_outside_vertex_array() {
    let positions = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
    let uv = [[0.0, 0.0]; 4];
    let bytes = fixture(&positions, &[face([0, 0, 0, 255], uv, [0, 1, 99, 0xffff], 0)]);

    let err = decode_o3d(&bytes, O3dLimits::default()).unwrap_err();
    assert!(matches!(err, O3dError::VertexIndexOutOfRange { index: 99, .. }));
}

#[test]
fn enforces_input_size_limit_before_parsing() {
    let bytes = vec![0u8; 32];
    let limits = O3dLimits {
        max_input_bytes: 16,
        ..O3dLimits::default()
    };

    let err = decode_o3d(&bytes, limits).unwrap_err();
    assert!(matches!(err, O3dError::InputLimitExceeded { .. }));
}

#[test]
fn rejects_declared_counts_over_limits() {
    let mut bytes = Vec::new();
    push_u32(&mut bytes, 10);
    push_u32(&mut bytes, 0);
    push_u32(&mut bytes, 0);
    push_u32(&mut bytes, 0);
    let limits = O3dLimits {
        max_vertices: 3,
        ..O3dLimits::default()
    };

    let err = decode_o3d(&bytes, limits).unwrap_err();
    assert!(matches!(err, O3dError::VertexCountLimit { count: 10, .. }));
}
RUST

cargo fmt --all
set +e
red_output="$(cargo test -p darkstone-o3d --test static_mesh 2>&1)"
red_rc=$?
set -e
printf '%s\n' "$red_output"
if (( red_rc == 0 )); then
  echo 'DARKSTONE_TASK4_RED=FAIL-test-passed-before-implementation' >&2
  exit 72
fi
if ! grep -Eq 'unresolved import|unresolved imports|cannot find' <<<"$red_output"; then
  echo 'DARKSTONE_TASK4_RED=FAIL-unexpected-failure' >&2
  exit 73
fi
printf 'DARKSTONE_TASK4_RED=PASS\n'

cat > crates/darkstone-o3d/src/lib.rs <<'RUST'
//! Bounds-checked decoder for Darkstone static O3D geometry.

mod parse;

pub use parse::{decode_o3d, O3dLimits};

use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum O3dError {
    #[error("O3D input size {size} exceeds configured limit {limit}")]
    InputLimitExceeded { size: usize, limit: usize },
    #[error("O3D declares {count} vertices, configured limit is {max}")]
    VertexCountLimit { count: u32, max: u32 },
    #[error("O3D declares {count} faces, configured limit is {max}")]
    FaceCountLimit { count: u32, max: u32 },
    #[error("arithmetic overflow while parsing O3D data")]
    ArithmeticOverflow,
    #[error("unexpected end of O3D data at byte offset {offset}")]
    UnexpectedEof { offset: usize },
    #[error("vertex {vertex} axis {axis} is not finite: {value}")]
    NonFinitePosition {
        vertex: usize,
        axis: usize,
        value: f32,
    },
    #[error("face {face} corner {corner} references vertex {index}, but only {vertex_count} vertices exist")]
    VertexIndexOutOfRange {
        face: usize,
        corner: usize,
        index: u16,
        vertex_count: usize,
    },
    #[error("too many neutral vertices to address with u32 indices")]
    NeutralVertexOverflow,
    #[error("too many distinct material slots to address with u16 indices")]
    MaterialSlotOverflow,
    #[error("neutral mesh validation failed: {0}")]
    Mesh(#[from] darkstone_assets::MeshError),
}
RUST

cat > crates/darkstone-o3d/src/parse.rs <<'RUST'
use darkstone_assets::{MaterialSlot, Mesh, MeshTriangle, MeshVertex};

use crate::O3dError;

const HEADER_BYTES: usize = 16;
const FACE_BYTES: usize = 50;
const UV_SCALE: f32 = 1.0 / 256.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct O3dLimits {
    pub max_vertices: u32,
    pub max_faces: u32,
    pub max_input_bytes: usize,
}

impl Default for O3dLimits {
    fn default() -> Self {
        Self {
            max_vertices: 5_000_000,
            max_faces: 5_000_000,
            max_input_bytes: 512 * 1024 * 1024,
        }
    }
}

pub fn decode_o3d(bytes: &[u8], limits: O3dLimits) -> Result<Mesh, O3dError> {
    if bytes.len() > limits.max_input_bytes {
        return Err(O3dError::InputLimitExceeded {
            size: bytes.len(),
            limit: limits.max_input_bytes,
        });
    }

    let mut at = 0usize;
    let vertex_count = take_u32(bytes, &mut at)?;
    let face_count = take_u32(bytes, &mut at)?;
    let _unknown_a = take_u32(bytes, &mut at)?;
    let _unknown_b = take_u32(bytes, &mut at)?;
    debug_assert_eq!(at, HEADER_BYTES);

    if vertex_count > limits.max_vertices {
        return Err(O3dError::VertexCountLimit {
            count: vertex_count,
            max: limits.max_vertices,
        });
    }
    if face_count > limits.max_faces {
        return Err(O3dError::FaceCountLimit {
            count: face_count,
            max: limits.max_faces,
        });
    }

    let vertex_count_usize = usize::try_from(vertex_count).map_err(|_| O3dError::ArithmeticOverflow)?;
    let face_count_usize = usize::try_from(face_count).map_err(|_| O3dError::ArithmeticOverflow)?;
    let vertex_bytes = vertex_count_usize
        .checked_mul(12)
        .ok_or(O3dError::ArithmeticOverflow)?;
    let face_bytes = face_count_usize
        .checked_mul(FACE_BYTES)
        .ok_or(O3dError::ArithmeticOverflow)?;
    let needed = HEADER_BYTES
        .checked_add(vertex_bytes)
        .and_then(|value| value.checked_add(face_bytes))
        .ok_or(O3dError::ArithmeticOverflow)?;
    if bytes.len() < needed {
        return Err(O3dError::UnexpectedEof { offset: bytes.len() });
    }

    let mut positions = Vec::with_capacity(vertex_count_usize);
    for vertex in 0..vertex_count_usize {
        let mut position = [0.0f32; 3];
        for (axis, slot) in position.iter_mut().enumerate() {
            let value = take_f32(bytes, &mut at)?;
            if !value.is_finite() {
                return Err(O3dError::NonFinitePosition {
                    vertex,
                    axis,
                    value,
                });
            }
            *slot = value;
        }
        positions.push(position);
    }

    let estimated_vertices = face_count_usize
        .checked_mul(4)
        .ok_or(O3dError::ArithmeticOverflow)?;
    let estimated_triangles = face_count_usize
        .checked_mul(2)
        .ok_or(O3dError::ArithmeticOverflow)?;
    let mut vertices = Vec::with_capacity(estimated_vertices);
    let mut triangles = Vec::with_capacity(estimated_triangles);
    let mut materials = Vec::<MaterialSlot>::new();

    for face_index in 0..face_count_usize {
        let bgra = take_array_4(bytes, &mut at)?;
        let color = [bgra[2], bgra[1], bgra[0], bgra[3]];

        let mut uv = [[0.0f32; 2]; 4];
        for pair in &mut uv {
            pair[0] = take_f32(bytes, &mut at)? * UV_SCALE;
            pair[1] = take_f32(bytes, &mut at)? * UV_SCALE;
        }

        let mut source_indices = [0u16; 4];
        for slot in &mut source_indices {
            *slot = take_u16(bytes, &mut at)?;
        }
        let _unknown = take_u32(bytes, &mut at)?;
        let material_number = take_u16(bytes, &mut at)?;

        let corner_count = if source_indices[3] == 0xffff { 3 } else { 4 };
        let material_slot = material_slot(&mut materials, material_number)?;
        let base = u32::try_from(vertices.len()).map_err(|_| O3dError::NeutralVertexOverflow)?;

        for corner in 0..corner_count {
            let source_index = source_indices[corner];
            let position = positions
                .get(usize::from(source_index))
                .copied()
                .ok_or(O3dError::VertexIndexOutOfRange {
                    face: face_index,
                    corner,
                    index: source_index,
                    vertex_count: positions.len(),
                })?;
            let mut vertex = MeshVertex::new(position);
            vertex.uv = uv[corner];
            vertex.color = color;
            vertex.normal = [0.0, 0.0, 0.0];
            vertices.push(vertex);
        }

        triangles.push(MeshTriangle::new([base, base + 1, base + 2], material_slot));
        if corner_count == 4 {
            triangles.push(MeshTriangle::new([base, base + 2, base + 3], material_slot));
        }
    }

    generate_normals(&mut vertices, &triangles);
    Mesh::new(vertices, triangles, materials).map_err(O3dError::from)
}

fn material_slot(materials: &mut Vec<MaterialSlot>, source_number: u16) -> Result<u16, O3dError> {
    if let Some(index) = materials
        .iter()
        .position(|material| material.source_number == source_number)
    {
        return u16::try_from(index).map_err(|_| O3dError::MaterialSlotOverflow);
    }
    let slot = u16::try_from(materials.len()).map_err(|_| O3dError::MaterialSlotOverflow)?;
    materials.push(MaterialSlot::new(source_number));
    Ok(slot)
}

fn generate_normals(vertices: &mut [MeshVertex], triangles: &[MeshTriangle]) {
    for triangle in triangles {
        let [a, b, c] = triangle.indices.map(|index| index as usize);
        let pa = vertices[a].position;
        let pb = vertices[b].position;
        let pc = vertices[c].position;
        let ab = sub(pb, pa);
        let ac = sub(pc, pa);
        let normal = cross(ab, ac);
        for index in [a, b, c] {
            vertices[index].normal[0] += normal[0];
            vertices[index].normal[1] += normal[1];
            vertices[index].normal[2] += normal[2];
        }
    }

    for vertex in vertices {
        let n = vertex.normal;
        let len_sq = n[0] * n[0] + n[1] * n[1] + n[2] * n[2];
        if len_sq.is_finite() && len_sq > f32::EPSILON {
            let inv_len = len_sq.sqrt().recip();
            vertex.normal = [n[0] * inv_len, n[1] * inv_len, n[2] * inv_len];
        } else {
            vertex.normal = [0.0, 1.0, 0.0];
        }
    }
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn take_u16(bytes: &[u8], at: &mut usize) -> Result<u16, O3dError> {
    let raw = take(bytes, at, 2)?;
    Ok(u16::from_le_bytes([raw[0], raw[1]]))
}

fn take_u32(bytes: &[u8], at: &mut usize) -> Result<u32, O3dError> {
    let raw = take(bytes, at, 4)?;
    Ok(u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]))
}

fn take_f32(bytes: &[u8], at: &mut usize) -> Result<f32, O3dError> {
    Ok(f32::from_bits(take_u32(bytes, at)?))
}

fn take_array_4(bytes: &[u8], at: &mut usize) -> Result<[u8; 4], O3dError> {
    let raw = take(bytes, at, 4)?;
    Ok([raw[0], raw[1], raw[2], raw[3]])
}

fn take<'a>(bytes: &'a [u8], at: &mut usize, count: usize) -> Result<&'a [u8], O3dError> {
    let end = at.checked_add(count).ok_or(O3dError::ArithmeticOverflow)?;
    let raw = bytes
        .get(*at..end)
        .ok_or(O3dError::UnexpectedEof { offset: *at })?;
    *at = end;
    Ok(raw)
}
RUST

cargo fmt --all
cargo test -p darkstone-o3d
cargo clippy -p darkstone-o3d --all-targets -- -D warnings
printf 'DARKSTONE_TASK4_GREEN=PASS\n'

if [[ "$in_git" == true ]]; then
  git add Cargo.toml crates/darkstone-o3d scripts/continue-through-task4-on-host.sh
  if ! git diff --cached --quiet; then
    git commit -m 'feat: decode Darkstone O3D static meshes'
  fi
  printf 'DARKSTONE_TASK4_COMMIT=%s\n' "$(git rev-parse --short HEAD)"
else
  printf 'DARKSTONE_TASK4_COMMIT=SKIPPED-standalone\n'
fi

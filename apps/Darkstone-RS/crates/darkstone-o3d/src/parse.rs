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

    let vertex_count_usize =
        usize::try_from(vertex_count).map_err(|_| O3dError::ArithmeticOverflow)?;
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
        return Err(O3dError::UnexpectedEof {
            offset: bytes.len(),
        });
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
            let position = positions.get(usize::from(source_index)).copied().ok_or(
                O3dError::VertexIndexOutOfRange {
                    face: face_index,
                    corner,
                    index: source_index,
                    vertex_count: positions.len(),
                },
            )?;
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

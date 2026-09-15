use darkstone_o3d::{O3dError, O3dLimits, decode_o3d};

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
    let bytes = fixture(
        &positions,
        &[face([0, 0, 0, 255], uv, [0, 1, 2, 0xffff], 0)],
    );

    let err = decode_o3d(&bytes, O3dLimits::default()).unwrap_err();
    assert!(matches!(err, O3dError::NonFinitePosition { .. }));
}

#[test]
fn rejects_face_index_outside_vertex_array() {
    let positions = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
    let uv = [[0.0, 0.0]; 4];
    let bytes = fixture(
        &positions,
        &[face([0, 0, 0, 255], uv, [0, 1, 99, 0xffff], 0)],
    );

    let err = decode_o3d(&bytes, O3dLimits::default()).unwrap_err();
    assert!(matches!(
        err,
        O3dError::VertexIndexOutOfRange { index: 99, .. }
    ));
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

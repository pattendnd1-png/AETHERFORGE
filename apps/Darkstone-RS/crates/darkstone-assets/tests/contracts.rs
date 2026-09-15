use std::path::PathBuf;

use darkstone_assets::{
    AssetStatus, CompatibilityReport, InstallCandidate, MaterialSlot, Mesh, MeshError,
    MeshTriangle, MeshVertex, SupportState,
};

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
    )
    .unwrap();

    assert_eq!(mesh.bounds.min, [-2.0, -5.0, 1.0]);
    assert_eq!(mesh.bounds.max, [3.0, 2.0, 9.0]);
}

#[test]
fn rejects_out_of_range_triangle_index() {
    let err = Mesh::new(
        vec![MeshVertex::new([0.0, 0.0, 0.0])],
        vec![MeshTriangle::new([0, 1, 0], 0)],
        vec![MaterialSlot::new(0)],
    )
    .unwrap_err();

    assert!(matches!(
        err,
        MeshError::VertexIndexOutOfRange { index: 1, .. }
    ));
}

#[test]
fn install_candidate_preserves_user_selected_path() {
    let path = PathBuf::from("/games/Darkstone");
    let candidate = InstallCandidate::explicit(path.clone());
    assert_eq!(candidate.path, path);
}

#[test]
fn compatibility_report_serializes_schema_and_asset_status() {
    let report = CompatibilityReport::new(
        PathBuf::from("/games/Darkstone"),
        vec![AssetStatus::new(
            "DATA.MTF",
            "MODEL\\TEST.O3D",
            123,
            SupportState::Supported,
        )],
    );

    let json = serde_json::to_value(report).unwrap();
    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["assets"][0]["archive_name"], "DATA.MTF");
    assert_eq!(json["assets"][0]["status"], "supported");
}

use std::path::PathBuf;

use aether_core::IndexState;
use aether_storage::{
    SqliteStore,
    models::{IndexChunkRecord, IndexedFileRecord},
};
use chrono::Utc;
use uuid::Uuid;

#[test]
fn vectors_are_keyed_by_model_and_content_hash_and_model_switch_preserves_chunks() {
    let root = std::env::temp_dir().join(format!("aetherai-task9-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let store = SqliteStore::open(root.join("db.sqlite")).unwrap();

    let file_id = Uuid::new_v4();
    let chunk_id = Uuid::new_v4();
    let file = IndexedFileRecord {
        id: file_id,
        attachment_id: Uuid::new_v4(),
        canonical_path: PathBuf::from("/project/a.txt"),
        size_bytes: 5,
        modified_ns: 1,
        content_hash: "file-hash".into(),
        index_version: 1,
        extractor_version: 1,
        state: IndexState::Indexed,
        last_indexed_at: Utc::now(),
        error: None,
    };
    let chunk = IndexChunkRecord {
        id: chunk_id,
        file_id,
        ordinal: 0,
        start_line: 1,
        end_line: 1,
        content: "alpha".into(),
        content_hash: "chunk-hash".into(),
        estimated_tokens: 2,
    };

    store
        .replace_file_index(&file, std::slice::from_ref(&chunk), &[], &[])
        .unwrap();
    store
        .save_embedding_vector(chunk_id, "model-a", "chunk-hash", &[1.0, 0.0])
        .unwrap();
    assert_eq!(
        store
            .embedding_vector(chunk_id, "model-a", "chunk-hash", 2)
            .unwrap(),
        Some(vec![1.0, 0.0])
    );
    assert!(
        store
            .embedding_vector(chunk_id, "model-a", "different-hash", 2)
            .unwrap()
            .is_none()
    );

    store
        .save_embedding_vector(chunk_id, "model-b", "chunk-hash", &[0.0, 1.0])
        .unwrap();
    let removed = store.invalidate_embedding_models_except("model-b").unwrap();
    assert_eq!(removed, 1);
    assert!(
        store
            .embedding_vector(chunk_id, "model-a", "chunk-hash", 2)
            .unwrap()
            .is_none()
    );
    assert_eq!(store.chunks_for_file(file_id).unwrap(), vec![chunk]);

    let _ = std::fs::remove_dir_all(root);
}

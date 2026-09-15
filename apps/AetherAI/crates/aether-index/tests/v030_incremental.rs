use std::path::Path;

use aether_index::{ChangeDecision, FileFingerprint, decide_change, sha256_bytes, sha256_hex};

#[test]
fn sha256_matches_known_vectors() {
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(
        sha256_hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(sha256_bytes(b"abc").len(), 32);
}

#[test]
fn unchanged_metadata_skips_reindex() {
    let old = FileFingerprint::new(128, 12345, "abc", 1, 1);
    let current = FileFingerprint::new(128, 12345, "abc", 1, 1);
    assert_eq!(
        decide_change(Some(&old), Some(&current)),
        ChangeDecision::Skip
    );
}

#[test]
fn changed_hash_reindexes_only_that_file() {
    let old = FileFingerprint::new(128, 12345, "old", 1, 1);
    let new = FileFingerprint::new(129, 12346, "new", 1, 1);
    assert_eq!(
        decide_change(Some(&old), Some(&new)),
        ChangeDecision::Reindex
    );
}

#[test]
fn missing_file_marks_remove_from_current_index() {
    let old = FileFingerprint::new(128, 12345, "old", 1, 1);
    assert_eq!(decide_change(Some(&old), None), ChangeDecision::Remove);
}

#[test]
fn new_file_requires_indexing() {
    let current = FileFingerprint::new(64, 999, "new", 1, 1);
    assert_eq!(decide_change(None, Some(&current)), ChangeDecision::Reindex);
}

#[test]
fn parser_version_change_requires_reindex_even_with_same_content() {
    let old = FileFingerprint::new(128, 12345, "same", 1, 1);
    let new = FileFingerprint::new(128, 12345, "same", 2, 1);
    assert_eq!(
        decide_change(Some(&old), Some(&new)),
        ChangeDecision::Reindex
    );
}

#[test]
fn equal_verified_hash_can_skip_timestamp_churn() {
    let old = FileFingerprint::new(128, 12345, "same", 1, 1);
    let new = FileFingerprint::new(128, 99999, "same", 1, 1);
    assert_eq!(decide_change(Some(&old), Some(&new)), ChangeDecision::Skip);
}

#[test]
fn restored_mtime_with_changed_content_reindexes_when_verified() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("same-size.txt");
    std::fs::write(&path, "AAAA").unwrap();

    let old = FileFingerprint::from_path(&path, 1, 1, true).unwrap();
    std::fs::write(&path, "BBBB").unwrap();
    let mut current = FileFingerprint::from_path(&path, 1, 1, true).unwrap();

    current.modified_ns = old.modified_ns;
    assert_eq!(current.size_bytes, old.size_bytes);
    assert_ne!(current.content_hash, old.content_hash);
    assert_eq!(
        decide_change(Some(&old), Some(&current)),
        ChangeDecision::Reindex
    );
}

#[test]
fn metadata_only_fingerprint_does_not_hash_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("metadata.txt");
    std::fs::write(&path, "metadata only\n").unwrap();

    let fingerprint = FileFingerprint::from_path(&path, 1, 1, false).unwrap();
    assert_eq!(fingerprint.content_hash, None);
    assert!(fingerprint.size_bytes > 0);
}

#[test]
fn approved_source_exposes_incremental_fingerprint_bridge() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("lib.rs");
    std::fs::write(&path, "pub fn task5() {}\n").unwrap();

    let sources = aether_index::enumerate_approved_sources(
        dir.path(),
        &[dir.path().to_path_buf()],
        &aether_index::IndexPolicy::default(),
    )
    .unwrap();

    let canonical = Path::new(&path).canonicalize().unwrap();
    let source = sources
        .iter()
        .find(|source| source.canonical_path == canonical)
        .unwrap();
    let fingerprint = source.fingerprint(1, 1, true).unwrap();

    assert_eq!(fingerprint.index_version, 1);
    assert_eq!(fingerprint.extractor_version, 1);
    assert!(fingerprint.content_hash.is_some());
}

#[test]
fn unavailable_state_remains_distinct_from_remove() {
    assert_ne!(ChangeDecision::Unavailable, ChangeDecision::Remove);
}

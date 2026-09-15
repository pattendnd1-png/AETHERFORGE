use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use aether_core::{
    AttachmentKind, AttachmentRecord, ConversationId, IndexState, PermissionDecision,
    PermissionKind, ProjectId,
};
use aether_index::{
    AttachmentPermissionEvaluator, AttachmentRegistry, AttachmentRepository, FileKind, IndexError,
    IndexPolicy, enumerate_approved_sources,
};
use uuid::Uuid;

#[derive(Clone, Default)]
struct TestAttachmentRepo(Arc<Mutex<Vec<AttachmentRecord>>>);

impl AttachmentRepository for TestAttachmentRepo {
    fn save_attachment(&self, record: &AttachmentRecord) -> Result<(), IndexError> {
        let mut records = self.0.lock().unwrap();
        if let Some(existing) = records
            .iter_mut()
            .find(|item| item.attachment_id == record.attachment_id)
        {
            *existing = record.clone();
        } else {
            records.push(record.clone());
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
struct FixedPermissions(PermissionDecision);

impl AttachmentPermissionEvaluator for FixedPermissions {
    fn decision(
        &self,
        _conversation: ConversationId,
        _project: Option<ProjectId>,
        _kind: PermissionKind,
        _path: &Path,
    ) -> PermissionDecision {
        self.0
    }
}

#[test]
fn blocked_root_is_not_registered_or_enumerated() {
    let root = tempfile::tempdir().unwrap();
    let repo = TestAttachmentRepo::default();
    let registry =
        AttachmentRegistry::new(repo.clone(), FixedPermissions(PermissionDecision::Blocked));

    let error = registry
        .attach_root(Uuid::nil(), None, root.path())
        .unwrap_err();

    assert!(matches!(error, IndexError::PermissionDenied(_)));
    assert!(repo.0.lock().unwrap().is_empty());
}

#[test]
fn allowed_root_is_canonicalized_and_preserves_conversation_identity() {
    let root = tempfile::tempdir().unwrap();
    let conversation_id = Uuid::new_v4();
    let repo = TestAttachmentRepo::default();
    let registry = AttachmentRegistry::new(
        repo.clone(),
        FixedPermissions(PermissionDecision::AllowThisProject),
    );

    let record = registry
        .attach_root(conversation_id, None, root.path())
        .unwrap();

    assert_eq!(record.conversation_id, conversation_id);
    assert_eq!(record.canonical_path, root.path().canonicalize().unwrap());
    assert_eq!(record.attachment_kind, AttachmentKind::ProjectRoot);
    assert_eq!(record.index_state, IndexState::Pending);
    assert_eq!(repo.0.lock().unwrap().as_slice(), &[record]);
}

#[cfg(unix)]
#[test]
fn symlink_outside_approved_root_is_rejected() {
    let approved = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("outside.rs"), "fn outside() {}\n").unwrap();
    std::os::unix::fs::symlink(outside.path(), approved.path().join("escape")).unwrap();

    let files = enumerate_approved_sources(
        approved.path(),
        &[approved.path().to_path_buf()],
        &IndexPolicy::default(),
    )
    .unwrap();

    assert!(
        files
            .iter()
            .all(|item| !item.canonical_path.starts_with(outside.path()))
    );
}

#[test]
fn default_policy_excludes_build_cache_and_secret_names() {
    let project = tempfile::tempdir().unwrap();
    for rel in [
        "src/lib.rs",
        "target/debug/build.log",
        "node_modules/pkg/index.js",
        ".git/objects/aa/blob",
        ".ssh/id_ed25519",
        ".env",
    ] {
        let path = project.path().join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, "fixture\n").unwrap();
    }

    let files = enumerate_approved_sources(
        project.path(),
        &[project.path().to_path_buf()],
        &IndexPolicy::default(),
    )
    .unwrap();

    let mut rel = files
        .into_iter()
        .map(|item| {
            item.canonical_path
                .strip_prefix(project.path().canonicalize().unwrap())
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect::<Vec<_>>();
    rel.sort();

    assert_eq!(rel, vec!["src/lib.rs"]);
}

#[test]
fn supported_file_detection_is_typed_and_binary_is_metadata_only() {
    let dir = tempfile::tempdir().unwrap();
    let rust = dir.path().join("lib.rs");
    let verify = dir.path().join("AetherAI-v0.3.0-VERIFY.txt");
    let binary = dir.path().join("blob.bin");

    std::fs::write(&rust, "pub fn typed() {}\n").unwrap();
    std::fs::write(&verify, "AETHERAI_TEST=PASS\n").unwrap();
    std::fs::write(&binary, [0_u8, 159, 146, 150]).unwrap();

    let files = enumerate_approved_sources(
        dir.path(),
        &[dir.path().to_path_buf()],
        &IndexPolicy::default(),
    )
    .unwrap();

    let by_name = |name: &str| {
        files
            .iter()
            .find(|item| item.canonical_path.file_name().unwrap() == name)
            .unwrap()
    };

    assert_eq!(by_name("lib.rs").kind, FileKind::Rust);
    assert!(!by_name("lib.rs").metadata_only);
    assert_eq!(
        by_name("AetherAI-v0.3.0-VERIFY.txt").kind,
        FileKind::VerificationText
    );
    assert_eq!(by_name("blob.bin").kind, FileKind::Unsupported);
    assert!(by_name("blob.bin").metadata_only);
}

#[test]
fn missing_attachment_is_preserved_and_marked_unavailable() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("notes.txt");
    std::fs::write(&source, "keep relationship\n").unwrap();

    let repo = TestAttachmentRepo::default();
    let registry = AttachmentRegistry::new(
        repo.clone(),
        FixedPermissions(PermissionDecision::AllowThisChat),
    );

    let mut record = registry.attach_file(Uuid::new_v4(), None, &source).unwrap();
    std::fs::remove_file(&source).unwrap();

    registry.refresh_attachment_state(&mut record).unwrap();

    assert_eq!(record.index_state, IndexState::Unavailable);
    assert_eq!(repo.0.lock().unwrap().len(), 1);
    assert_eq!(
        repo.0.lock().unwrap()[0].attachment_id,
        record.attachment_id
    );
}

#[test]
fn user_ignore_is_applied_without_glob_dependency() {
    let dir = tempfile::tempdir().unwrap();
    for rel in ["src/keep.rs", "generated/drop.rs"] {
        let path = dir.path().join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, "fixture\n").unwrap();
    }

    let policy = IndexPolicy {
        user_ignores: vec!["generated".into()],
        ..IndexPolicy::default()
    };
    let files =
        enumerate_approved_sources(dir.path(), &[dir.path().to_path_buf()], &policy).unwrap();

    assert_eq!(files.len(), 1);
    assert!(
        files[0]
            .canonical_path
            .ends_with(PathBuf::from("src/keep.rs"))
    );
}

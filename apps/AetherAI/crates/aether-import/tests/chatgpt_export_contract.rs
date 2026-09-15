use aether_import::{CHATGPT_IMPORT_SCHEMA_VERSION, ImportError};

#[test]
fn import_schema_version_is_explicit_and_stable() {
    assert_eq!(CHATGPT_IMPORT_SCHEMA_VERSION, 1);
}

#[test]
fn archive_errors_remain_typed_at_import_boundary() {
    let error = ImportError::Archive("fixture".into());
    assert_eq!(error.to_string(), "archive error: fixture");
}

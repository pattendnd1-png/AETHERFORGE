use std::fs;
use std::path::PathBuf;

#[test]
fn gguf_provider_delegates_tokenization_to_local_runtime_client() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = fs::read_to_string(root.join("src/lib.rs")).unwrap();
    assert!(source.contains("self.client.tokenize(input)"));
    assert!(!source.contains("GGUF runtime tokenization endpoint is not configured"));
}

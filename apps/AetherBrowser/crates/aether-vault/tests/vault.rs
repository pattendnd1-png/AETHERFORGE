use aether_vault::{
    VaultError, VaultRecord, VaultRecordKind, open_records, origin_matches, seal_records,
};

fn sample() -> Vec<VaultRecord> {
    vec![VaultRecord {
        id: "login-1".into(),
        kind: VaultRecordKind::WebsitePassword,
        username: Some("user@example.com".into()),
        secret: "correct horse battery staple".into(),
        origin: Some("https://example.com".into()),
        provider: None,
    }]
}

#[test]
fn encrypted_vault_round_trips() {
    let sealed = seal_records(b"vault-password", &sample()).unwrap();
    assert_eq!(open_records(b"vault-password", &sealed).unwrap(), sample());
}

#[test]
fn wrong_password_is_rejected() {
    let sealed = seal_records(b"vault-password", &sample()).unwrap();
    assert_eq!(open_records(b"wrong", &sealed), Err(VaultError::Decryption));
}

#[test]
fn tampering_is_rejected() {
    let mut sealed = seal_records(b"vault-password", &sample()).unwrap();
    sealed.ciphertext[0] ^= 1;
    assert_eq!(
        open_records(b"vault-password", &sealed),
        Err(VaultError::Decryption)
    );
}

#[test]
fn autofill_origin_requires_same_origin() {
    assert!(origin_matches(
        "https://example.com",
        "https://example.com/login"
    ));
    assert!(!origin_matches(
        "https://example.com",
        "https://evil.example/login"
    ));
    assert!(!origin_matches(
        "https://example.com",
        "http://example.com/login"
    ));
}

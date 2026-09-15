#![forbid(unsafe_code)]
//! Encrypted Aether password, passkey-reference, OAuth-token, and streaming-secret vault.

use argon2::Argon2;
use chacha20poly1305::{
    XChaCha20Poly1305, XNonce,
    aead::{Aead, KeyInit},
};
use serde::{Deserialize, Serialize};
use url::Url;

const VAULT_VERSION: u8 = 1;
const SALT_BYTES: usize = 16;
const NONCE_BYTES: usize = 24;
const KEY_BYTES: usize = 32;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum VaultRecordKind {
    WebsitePassword,
    PasskeyReference,
    OAuthToken,
    StreamKey,
    RecoveryCode,
    SecureNote,
    AutofillIdentity,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct VaultRecord {
    pub id: String,
    pub kind: VaultRecordKind,
    pub username: Option<String>,
    pub secret: String,
    pub origin: Option<String>,
    pub provider: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EncryptedVault {
    pub version: u8,
    pub salt: [u8; SALT_BYTES],
    pub nonce: [u8; NONCE_BYTES],
    pub ciphertext: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VaultError {
    Randomness,
    KeyDerivation,
    Serialization,
    Decryption,
    UnsupportedVersion,
}

fn derive_key(password: &[u8], salt: &[u8; SALT_BYTES]) -> Result<[u8; KEY_BYTES], VaultError> {
    let mut key = [0_u8; KEY_BYTES];
    Argon2::default()
        .hash_password_into(password, salt, &mut key)
        .map_err(|_| VaultError::KeyDerivation)?;
    Ok(key)
}

pub fn seal_records(
    password: &[u8],
    records: &[VaultRecord],
) -> Result<EncryptedVault, VaultError> {
    let mut salt = [0_u8; SALT_BYTES];
    let mut nonce = [0_u8; NONCE_BYTES];
    getrandom::getrandom(&mut salt).map_err(|_| VaultError::Randomness)?;
    getrandom::getrandom(&mut nonce).map_err(|_| VaultError::Randomness)?;
    let key = derive_key(password, &salt)?;
    let cipher = XChaCha20Poly1305::new((&key).into());
    let plaintext = serde_json::to_vec(records).map_err(|_| VaultError::Serialization)?;
    let ciphertext = cipher
        .encrypt(XNonce::from_slice(&nonce), plaintext.as_ref())
        .map_err(|_| VaultError::Decryption)?;
    Ok(EncryptedVault {
        version: VAULT_VERSION,
        salt,
        nonce,
        ciphertext,
    })
}

pub fn open_records(
    password: &[u8],
    vault: &EncryptedVault,
) -> Result<Vec<VaultRecord>, VaultError> {
    if vault.version != VAULT_VERSION {
        return Err(VaultError::UnsupportedVersion);
    }
    let key = derive_key(password, &vault.salt)?;
    let cipher = XChaCha20Poly1305::new((&key).into());
    let plaintext = cipher
        .decrypt(XNonce::from_slice(&vault.nonce), vault.ciphertext.as_ref())
        .map_err(|_| VaultError::Decryption)?;
    serde_json::from_slice(&plaintext).map_err(|_| VaultError::Serialization)
}

#[must_use]
pub fn origin_matches(stored_origin: &str, candidate_url: &str) -> bool {
    let Ok(stored) = Url::parse(stored_origin) else {
        return false;
    };
    let Ok(candidate) = Url::parse(candidate_url) else {
        return false;
    };
    stored.scheme() == candidate.scheme()
        && stored.host_str() == candidate.host_str()
        && stored.port_or_known_default() == candidate.port_or_known_default()
}

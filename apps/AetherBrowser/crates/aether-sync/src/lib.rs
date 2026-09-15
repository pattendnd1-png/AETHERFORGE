#![forbid(unsafe_code)]
//! Provider-neutral encrypted-sync record contracts.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SyncRecordKind {
    Bookmark,
    Preference,
    Workspace,
    OpenTab,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SyncRecord {
    pub id: String,
    pub kind: SyncRecordKind,
    pub encrypted_payload: Vec<u8>,
    pub revision: u64,
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AssetId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetKind {
    Texture,
    Mesh,
    Animation,
    Audio,
    World,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetHandle {
    pub id: AssetId,
    pub kind: AssetKind,
    pub source_key: String,
}

impl AssetHandle {
    pub fn metadata(id: u64, source_key: impl Into<String>) -> Self {
        Self {
            id: AssetId(id),
            kind: AssetKind::Unknown,
            source_key: source_key.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_handle_does_not_claim_decoded_asset_kind() {
        let handle = AssetHandle::metadata(7, "synthetic-key");
        assert_eq!(handle.kind, AssetKind::Unknown);
    }
}

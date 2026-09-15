use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelCatalogEntry {
    pub id: String,
    pub display_name: String,
    pub repository: String,
    pub revision: String,
    pub file: String,
    pub download_url: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub license: String,
    pub context_tokens: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelCatalog {
    pub models: Vec<ModelCatalogEntry>,
}

#[derive(Debug, Error)]
pub enum CatalogError {
    #[error("model catalog JSON is invalid: {0}")]
    Json(#[from] serde_json::Error),
    #[error("model catalog entry {0} has no license")]
    MissingLicense(String),
    #[error("model catalog entry {0} has an invalid SHA-256")]
    InvalidSha256(String),
    #[error("model catalog entry {0} is not HTTPS")]
    InsecureUrl(String),
}
impl ModelCatalog {
    pub fn parse(json: &str) -> Result<Self, CatalogError> {
        let catalog: Self = serde_json::from_str(json)?;
        for model in &catalog.models {
            if model.license.trim().is_empty() {
                return Err(CatalogError::MissingLicense(model.id.clone()));
            }
            if model.sha256.len() != 64 || !model.sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err(CatalogError::InvalidSha256(model.id.clone()));
            }
            if !model.download_url.starts_with("https://") {
                return Err(CatalogError::InsecureUrl(model.id.clone()));
            }
        }
        Ok(catalog)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalog_requires_license() {
        let j = r#"{"models":[{"id":"x","display_name":"X","repository":"r","revision":"rev","file":"x.gguf","download_url":"https://example/x","sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","size_bytes":1,"license":"","context_tokens":1}]}"#;
        assert!(matches!(
            ModelCatalog::parse(j),
            Err(CatalogError::MissingLicense(_))
        ));
    }
}

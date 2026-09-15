#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

use crate::ImportError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedChatGptExport {
    pub root: PathBuf,
    pub entries: Vec<String>,
}

pub fn extract_chatgpt_export(
    archive: &Path,
    destination: &Path,
) -> Result<ExtractedChatGptExport, ImportError> {
    let entries = aether_system::list_zip_archive(archive)?;
    aether_system::extract_zip_archive(archive, destination)?;
    Ok(ExtractedChatGptExport {
        root: destination.to_path_buf(),
        entries,
    })
}

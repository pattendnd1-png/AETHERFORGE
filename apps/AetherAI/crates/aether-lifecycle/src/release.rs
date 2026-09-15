use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileDigest {
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleaseManifest {
    pub schema: u32,
    pub version: String,
    pub architecture: String,
    pub binary: String,
    pub browser_webview: bool,
    pub openai_api_required: bool,
    pub paid_service_required: bool,
    pub files: Vec<FileDigest>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseInfo {
    pub version: String,
    pub architecture: String,
    pub source: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedRelease {
    pub root: PathBuf,
    pub manifest: ReleaseManifest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationReport {
    pub version: String,
    pub architecture: String,
    pub checks: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateStatus {
    NotInstalled,
    UpToDate { current: String },
    Available(ReleaseInfo),
}

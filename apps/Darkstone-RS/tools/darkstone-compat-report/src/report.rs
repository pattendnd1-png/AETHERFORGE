use std::{fs::File, io, path::Path};

use darkstone_assets::CompatibilityReport;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ReportWriteError {
    #[error("failed to create report {path}: {source}")]
    Create {
        path: String,
        #[source]
        source: io::Error,
    },
    #[error("failed to serialize report: {0}")]
    Serialize(#[from] serde_json::Error),
}

pub fn write_json_report(
    report: &CompatibilityReport,
    path: &Path,
) -> Result<(), ReportWriteError> {
    let file = File::create(path).map_err(|source| ReportWriteError::Create {
        path: path.display().to_string(),
        source,
    })?;
    serde_json::to_writer_pretty(file, report)?;
    Ok(())
}

use reforge_core::{diagnostics::redact_snapshot, paths, DiagnosticSnapshot};
use std::fs;

pub fn export(snapshot: DiagnosticSnapshot, include_device_identifiers: bool) -> Result<std::path::PathBuf, String> {
    let dir = paths::diagnostic_export_dir();
    fs::create_dir_all(&dir).map_err(|error| format!("failed to create {}: {error}", dir.display()))?;
    let path = dir.join("reforge-diagnostics.json");
    let bundle = redact_snapshot(snapshot, include_device_identifiers);
    let json = serde_json::to_string_pretty(&bundle)
        .map_err(|error| format!("failed to encode diagnostics: {error}"))?;
    fs::write(&path, json).map_err(|error| format!("failed to write {}: {error}", path.display()))?;
    Ok(path)
}

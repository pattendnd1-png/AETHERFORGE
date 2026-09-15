use sanctuary_core::{InstallState, SanctuaryError};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallProbe {
    pub path: PathBuf,
    pub state: InstallState,
    pub build_info: String,
}

pub fn probe_install(path: &Path) -> Result<InstallProbe, SanctuaryError> {
    if !path.is_dir() {
        return Err(SanctuaryError::InstallationNotFound(path.to_path_buf()));
    }
    let build_info_path = path.join(".build.info");
    if !build_info_path.is_file() {
        return Err(SanctuaryError::ContentStoreUnreadable(format!(
            "{} is missing",
            build_info_path.display()
        )));
    }
    let data_path = path.join("Data");
    if !data_path.is_dir() {
        return Err(SanctuaryError::ContentStoreUnreadable(format!(
            "{} is missing",
            data_path.display()
        )));
    }
    fs::read_dir(&data_path).map_err(|e| {
        SanctuaryError::ContentStoreUnreadable(format!("cannot read {}: {e}", data_path.display()))
    })?;
    let build_info = fs::read_to_string(&build_info_path).map_err(|e| {
        SanctuaryError::ContentStoreUnreadable(format!(
            "cannot read {}: {e}",
            build_info_path.display()
        ))
    })?;
    Ok(InstallProbe {
        path: path.to_path_buf(),
        state: InstallState::FoundUnindexed,
        build_info,
    })
}

pub fn discover_candidates(home: &Path) -> Vec<PathBuf> {
    let candidates = [
        home.join("Games/Diablo III"),
        home.join("games/Diablo III"),
        home.join(".local/share/opensanctuary/imports/Diablo III"),
        home.join("Documents/Diablo III"),
    ];
    candidates.into_iter().filter(|p| p.is_dir()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_install() -> tempfile::TempDir {
        let td = tempfile::tempdir().unwrap();
        fs::write(
            td.path().join(".build.info"),
            "Branch!STRING:0|Build Key!HEX:16\nD3|00112233445566778899aabbccddeeff\n",
        )
        .unwrap();
        fs::create_dir(td.path().join("Data")).unwrap();
        td
    }

    #[test]
    fn accepts_install_with_build_info_and_data_dir() {
        let td = fixture_install();
        let probe = probe_install(td.path()).unwrap();
        assert_eq!(probe.state, InstallState::FoundUnindexed);
        assert!(probe.build_info.contains("Build Key"));
    }

    #[test]
    fn never_requires_diablo_exe() {
        let td = fixture_install();
        assert!(probe_install(td.path()).is_ok());
        assert!(!td.path().join("Diablo III.exe").exists());
    }

    #[test]
    fn missing_build_info_is_actionable() {
        let td = tempfile::tempdir().unwrap();
        fs::create_dir(td.path().join("Data")).unwrap();
        let err = probe_install(td.path()).unwrap_err().to_string();
        assert!(err.contains(".build.info"));
    }
}

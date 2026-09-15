use sanctuary_casc::probe_content_store;
use sanctuary_core::{LaunchDescriptor, SanctuaryError};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineRuntimeInfo {
    pub install_path: PathBuf,
    pub build_key: Option<String>,
    pub index_files: usize,
    pub data_files: usize,
    pub vulkan_available: bool,
    pub pipewire_available: bool,
}

pub fn load_descriptor(path: &Path) -> Result<LaunchDescriptor, SanctuaryError> {
    let bytes = fs::read(path).map_err(|source| SanctuaryError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let descriptor: LaunchDescriptor = serde_json::from_slice(&bytes)
        .map_err(|e| SanctuaryError::InvalidLaunchDescriptor(e.to_string()))?;
    descriptor.validate()?;
    Ok(descriptor)
}

pub fn run_descriptor(path: &Path) -> Result<EngineRuntimeInfo, SanctuaryError> {
    let descriptor = load_descriptor(path)?;
    let summary = probe_content_store(&descriptor.install_path)?;
    Ok(EngineRuntimeInfo {
        install_path: descriptor.install_path,
        build_key: summary.build_key,
        index_files: summary.index_files,
        data_files: summary.data_files,
        vulkan_available: sanctuary_render::detect().available,
        pipewire_available: sanctuary_audio::detect().pipewire_available,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sanctuary_core::LaunchDescriptor;

    fn fixture_install() -> tempfile::TempDir {
        let td = tempfile::tempdir().unwrap();
        fs::write(
            td.path().join(".build.info"),
            "Branch!STRING:0|Build Key!HEX:16\nD3|00112233445566778899aabbccddeeff\n",
        )
        .unwrap();
        let data = td.path().join("Data");
        fs::create_dir(&data).unwrap();
        fs::write(data.join("data.000"), b"synthetic").unwrap();
        fs::write(data.join("00.idx"), b"index").unwrap();
        td
    }

    #[test]
    fn loads_valid_descriptor() {
        let td = fixture_install();
        let descriptor = LaunchDescriptor::new(td.path().to_path_buf());
        let path = td.path().join("launch.json");
        fs::write(&path, serde_json::to_vec(&descriptor).unwrap()).unwrap();
        assert!(load_descriptor(&path).unwrap().validate().is_ok());
    }

    #[test]
    fn runtime_info_never_needs_windows_executable() {
        let td = fixture_install();
        let descriptor = LaunchDescriptor::new(td.path().to_path_buf());
        let path = td.path().join("launch.json");
        fs::write(&path, serde_json::to_vec(&descriptor).unwrap()).unwrap();
        let info = run_descriptor(&path).unwrap();
        assert_eq!(info.index_files, 1);
        assert!(!td.path().join("Diablo III.exe").exists());
    }
}

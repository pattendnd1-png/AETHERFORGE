use crate::FirmwareError;
use forgehx_core::DeviceId;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedFirmware {
    pub staged_id: String,
    pub device_id: DeviceId,
    pub original_filename: String,
    pub path: PathBuf,
    pub sha256: String,
    pub size_bytes: u64,
}

pub struct FirmwareStager {
    root: PathBuf,
}

impl FirmwareStager {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn stage(
        &self,
        device_id: &DeviceId,
        source: &Path,
    ) -> Result<StagedFirmware, FirmwareError> {
        let metadata =
            fs::symlink_metadata(source).map_err(|e| FirmwareError::Io(e.to_string()))?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(FirmwareError::NotRegularFile(source.display().to_string()));
        }
        let mut input = File::open(source).map_err(|e| FirmwareError::Io(e.to_string()))?;
        fs::create_dir_all(&self.root).map_err(|e| FirmwareError::Io(e.to_string()))?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| FirmwareError::Io(e.to_string()))?
            .as_nanos();
        let staged_id = format!("fw-{}-{now:x}", std::process::id());
        let destination = self.root.join(format!("{staged_id}.img"));
        let mut output = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&destination)
            .map_err(|e| FirmwareError::Io(e.to_string()))?;
        let mut hasher = Sha256::new();
        let mut size = 0u64;
        let mut buffer = [0u8; 64 * 1024];
        loop {
            let read = input
                .read(&mut buffer)
                .map_err(|e| FirmwareError::Io(e.to_string()))?;
            if read == 0 {
                break;
            }
            output
                .write_all(&buffer[..read])
                .map_err(|e| FirmwareError::Io(e.to_string()))?;
            hasher.update(&buffer[..read]);
            size += read as u64;
        }
        output
            .sync_all()
            .map_err(|e| FirmwareError::Io(e.to_string()))?;
        Ok(StagedFirmware {
            staged_id,
            device_id: device_id.clone(),
            original_filename: source
                .file_name()
                .and_then(|v| v.to_str())
                .unwrap_or("firmware")
                .to_owned(),
            path: destination,
            sha256: format!("{:x}", hasher.finalize()),
            size_bytes: size,
        })
    }

    pub fn forget(&self, staged: &StagedFirmware) -> Result<(), FirmwareError> {
        if staged.path.parent() != Some(self.root.as_path()) {
            return Err(FirmwareError::Io(
                "staged firmware is outside the ForgeHX staging root".into(),
            ));
        }
        fs::remove_file(&staged.path).map_err(|e| FirmwareError::Io(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> PathBuf {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("forgehx-fw-stage-{}-{n}", std::process::id()))
    }

    #[test]
    fn staged_copy_is_immutable_from_source_changes() {
        let root = root();
        fs::create_dir_all(&root).unwrap();
        let source = root.join("source.bin");
        fs::write(&source, b"original firmware bytes").unwrap();
        let stager = FirmwareStager::new(root.join("staged"));
        let staged = stager.stage(&DeviceId("mic".into()), &source).unwrap();
        fs::write(&source, b"changed").unwrap();
        assert_eq!(fs::read(&staged.path).unwrap(), b"original firmware bytes");
        assert_eq!(staged.size_bytes, 23);
        let _ = fs::remove_dir_all(root);
    }
}

use crate::{
    archive::ArchiveReader,
    blte::{BlteDecodeOptions, decode_blte_with_options},
    build::{BuildConfig, BuildInfo, load_build_config, parse_build_info},
    index::LocalIndex,
    key::EncodingKey,
};
use sanctuary_core::SanctuaryError;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct CascStorage {
    install_path: PathBuf,
    build_info: BuildInfo,
    build_config: BuildConfig,
    index: LocalIndex,
    archive: ArchiveReader,
    decode_options: BlteDecodeOptions,
}

impl CascStorage {
    pub fn open(install_path: &Path) -> Result<Self, SanctuaryError> {
        Self::open_with_options(install_path, BlteDecodeOptions::default())
    }

    pub fn open_with_options(
        install_path: &Path,
        decode_options: BlteDecodeOptions,
    ) -> Result<Self, SanctuaryError> {
        if !install_path.is_dir() {
            return Err(SanctuaryError::InstallationNotFound(
                install_path.to_path_buf(),
            ));
        }
        let build_info_path = install_path.join(".build.info");
        let build_text =
            fs::read_to_string(&build_info_path).map_err(|source| SanctuaryError::Io {
                path: build_info_path.clone(),
                source,
            })?;
        let build_info = parse_build_info(&build_text)?;
        let build_config = load_build_config(install_path, &build_info)?;
        let index = LocalIndex::open(install_path)?;
        Ok(Self {
            install_path: install_path.to_path_buf(),
            build_info,
            build_config,
            index,
            archive: ArchiveReader::new(install_path),
            decode_options,
        })
    }

    pub fn read_encoding_key(&self, key: EncodingKey) -> Result<Vec<u8>, SanctuaryError> {
        let location = self.index.lookup(&key).ok_or_else(|| {
            SanctuaryError::ContentStoreUnreadable(format!(
                "encoding key {key} is not present in the local CASC index"
            ))
        })?;
        let encoded = self.archive.read_blte(key.prefix(), location)?;
        decode_blte_with_options(&encoded, self.decode_options)
    }

    pub fn read_build_blob(&self, field: &str) -> Result<Vec<u8>, SanctuaryError> {
        let key = self.build_config.encoding_key(field)?.ok_or_else(|| {
            SanctuaryError::ContentStoreUnreadable(format!("build config has no '{field}' field"))
        })?;
        self.read_encoding_key(key)
    }

    pub fn install_path(&self) -> &Path {
        &self.install_path
    }

    pub fn build_info(&self) -> &BuildInfo {
        &self.build_info
    }

    pub fn build_config(&self) -> &BuildConfig {
        &self.build_config
    }

    pub fn index_entries(&self) -> usize {
        self.index.entry_count()
    }

    pub fn selected_index_files(&self) -> &[PathBuf] {
        self.index.selected_files()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::archive::ARCHIVE_ENVELOPE_BYTES;
    use std::fs;

    fn guarded_index(prefix: [u8; 9], encoded_size: usize) -> Vec<u8> {
        let header = [7u8; 16];
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&(header.len() as i32).to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&header);
        while bytes.len() % 16 != 0 {
            bytes.push(0);
        }
        bytes.extend_from_slice(&18i32.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        let mut entry = [0u8; 18];
        entry[..9].copy_from_slice(&prefix);
        entry[9] = 0;
        entry[10..14].copy_from_slice(&0u32.to_be_bytes());
        entry[14..18].copy_from_slice(&(encoded_size as i32).to_le_bytes());
        bytes.extend_from_slice(&entry);
        bytes
    }

    fn fixture() -> (tempfile::TempDir, EncodingKey) {
        let td = tempfile::tempdir().unwrap();
        let build_key = "00112233445566778899aabbccddeeff";
        let encoding_key = EncodingKey::parse_hex("11223344556677889900aabbccddeeff").unwrap();
        fs::write(
            td.path().join(".build.info"),
            format!("Branch!STRING:0|Build Key!HEX:16|Product!STRING:0\nD3|{build_key}|d3\n"),
        )
        .unwrap();
        let config_path = td.path().join("Data/config/00/11").join(build_key);
        fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        fs::write(
            &config_path,
            format!(
                "encoding = ffeeddccbbaa99887766554433221100 {}\n",
                encoding_key.to_hex()
            ),
        )
        .unwrap();

        let payload = b"native-casc-read";
        let mut blte = b"BLTE\0\0\0\0N".to_vec();
        blte.extend_from_slice(payload);
        let encoded_size = ARCHIVE_ENVELOPE_BYTES + blte.len();
        let data_root = td.path().join("Data/data");
        fs::create_dir_all(&data_root).unwrap();
        fs::write(
            data_root.join("0100000001.idx"),
            guarded_index(*encoding_key.prefix().as_bytes(), encoded_size),
        )
        .unwrap();

        let mut archive = Vec::with_capacity(encoded_size);
        archive.extend_from_slice(encoding_key.as_bytes());
        archive.extend_from_slice(&(encoded_size as i32).to_le_bytes());
        archive.extend_from_slice(&[0u8; 10]);
        archive.extend_from_slice(&blte);
        fs::write(data_root.join("data.000"), archive).unwrap();
        (td, encoding_key)
    }

    #[test]
    fn reads_build_blob_end_to_end_without_modifying_installation() {
        let (td, encoding_key) = fixture();
        let before = fs::metadata(td.path().join("Data/data/data.000"))
            .unwrap()
            .len();
        let storage = CascStorage::open(td.path()).unwrap();
        assert_eq!(storage.index_entries(), 1);
        assert_eq!(
            storage.read_encoding_key(encoding_key).unwrap(),
            b"native-casc-read"
        );
        assert_eq!(
            storage.read_build_blob("encoding").unwrap(),
            b"native-casc-read"
        );
        let after = fs::metadata(td.path().join("Data/data/data.000"))
            .unwrap()
            .len();
        assert_eq!(before, after);
    }

    #[test]
    fn missing_encoding_key_is_reported() {
        let (td, _) = fixture();
        let storage = CascStorage::open(td.path()).unwrap();
        let missing = EncodingKey::parse_hex("ffeeddccbbaa00998877665544332211").unwrap();
        assert!(storage.read_encoding_key(missing).is_err());
    }
}

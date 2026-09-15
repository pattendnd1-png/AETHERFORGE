use crate::key::EncodingKey;
use sanctuary_core::SanctuaryError;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct BuildInfo {
    pub branch: Option<String>,
    pub build_key: Option<String>,
    pub cdn_key: Option<String>,
    pub version: Option<String>,
    pub product: Option<String>,
    pub unknown: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BuildConfig {
    fields: BTreeMap<String, Vec<String>>,
}

impl BuildConfig {
    pub fn parse(text: &str) -> Result<Self, SanctuaryError> {
        let mut fields = BTreeMap::new();
        for raw_line in text.lines() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((raw_name, raw_value)) = line.split_once('=') else {
                return Err(SanctuaryError::ContentStoreUnreadable(format!(
                    "build config line has no '=' separator: {line}"
                )));
            };
            let name = raw_name.trim().to_ascii_lowercase();
            if name.is_empty() {
                return Err(SanctuaryError::ContentStoreUnreadable(
                    "build config contains an empty field name".into(),
                ));
            }
            let values = raw_value
                .split_whitespace()
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>();
            fields.insert(name, values);
        }
        Ok(Self { fields })
    }

    pub fn values(&self, name: &str) -> Option<&[String]> {
        self.fields
            .get(&name.to_ascii_lowercase())
            .map(Vec::as_slice)
    }

    pub fn encoding_key(&self, name: &str) -> Result<Option<EncodingKey>, SanctuaryError> {
        let Some(values) = self.values(name) else {
            return Ok(None);
        };
        let Some(value) = values.get(1) else {
            return Err(SanctuaryError::ContentStoreUnreadable(format!(
                "build config field '{name}' has no encoding key"
            )));
        };
        EncodingKey::parse_hex(value).map(Some)
    }

    pub fn fields(&self) -> &BTreeMap<String, Vec<String>> {
        &self.fields
    }
}

pub fn parse_build_info(build_info: &str) -> Result<BuildInfo, SanctuaryError> {
    let mut lines = build_info.lines().filter(|line| !line.trim().is_empty());
    let header_line = lines.next().ok_or_else(|| {
        SanctuaryError::ContentStoreUnreadable(".build.info has no header row".into())
    })?;
    let headers = header_line
        .split('|')
        .map(normalize_header)
        .collect::<Vec<_>>();
    if headers.is_empty() || headers.iter().all(String::is_empty) {
        return Err(SanctuaryError::ContentStoreUnreadable(
            ".build.info has no usable header columns".into(),
        ));
    }

    let data_line = lines.next().ok_or_else(|| {
        SanctuaryError::ContentStoreUnreadable(".build.info has no data row".into())
    })?;
    let values = data_line.split('|').map(str::trim).collect::<Vec<_>>();

    let mut result = BuildInfo::default();
    for (index, header) in headers.iter().enumerate() {
        if header.is_empty() {
            continue;
        }
        let value = values.get(index).copied().unwrap_or_default().to_string();
        match canonical_header(header).as_str() {
            "branch" => set_nonempty(&mut result.branch, &value),
            "buildkey" => set_nonempty(&mut result.build_key, &value),
            "cdnkey" => set_nonempty(&mut result.cdn_key, &value),
            "version" => set_nonempty(&mut result.version, &value),
            "product" => set_nonempty(&mut result.product, &value),
            _ => {
                result.unknown.insert(header.clone(), value);
            }
        }
    }
    Ok(result)
}

pub fn extract_build_key(build_info: &str) -> Option<String> {
    parse_build_info(build_info).ok()?.build_key
}

pub fn build_config_path(install_path: &Path, build_key: &str) -> Result<PathBuf, SanctuaryError> {
    let normalized = build_key.trim().to_ascii_lowercase();
    if normalized.len() < 4 || !normalized.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(SanctuaryError::ContentStoreUnreadable(format!(
            "invalid build key '{build_key}'"
        )));
    }
    Ok(install_path
        .join("Data/config")
        .join(&normalized[0..2])
        .join(&normalized[2..4])
        .join(normalized))
}

pub fn load_build_config(
    install_path: &Path,
    build_info: &BuildInfo,
) -> Result<BuildConfig, SanctuaryError> {
    let build_key = build_info.build_key.as_deref().ok_or_else(|| {
        SanctuaryError::ContentStoreUnreadable(".build.info has no build key".into())
    })?;
    let path = build_config_path(install_path, build_key)?;
    let text = fs::read_to_string(&path).map_err(|source| SanctuaryError::Io {
        path: path.clone(),
        source,
    })?;
    BuildConfig::parse(&text)
}

fn normalize_header(header: &str) -> String {
    header
        .split('!')
        .next()
        .unwrap_or(header)
        .trim()
        .to_string()
}

fn canonical_header(header: &str) -> String {
    header
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn set_nonempty(slot: &mut Option<String>, value: &str) {
    if !value.is_empty() {
        *slot = Some(value.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_build_config_and_resolves_encoding_key() {
        let config = BuildConfig::parse(
            "# synthetic\nencoding = 00112233445566778899aabbccddeeff ffeeddccbbaa99887766554433221100\nroot = aabbccddeeff00112233445566778899 0123456789abcdeffedcba9876543210\n",
        )
        .unwrap();
        assert_eq!(
            config.encoding_key("encoding").unwrap().unwrap().to_hex(),
            "ffeeddccbbaa99887766554433221100"
        );
        assert_eq!(config.values("root").unwrap().len(), 2);
    }

    #[test]
    fn build_config_path_uses_casc_shards() {
        let path =
            build_config_path(Path::new("/game"), "00112233445566778899aabbccddeeff").unwrap();
        assert_eq!(
            path,
            PathBuf::from("/game/Data/config/00/11/00112233445566778899aabbccddeeff")
        );
    }

    #[test]
    fn malformed_build_config_is_rejected() {
        assert!(BuildConfig::parse("no separator here").is_err());
        assert!(build_config_path(Path::new("/game"), "bad-key").is_err());
    }
}

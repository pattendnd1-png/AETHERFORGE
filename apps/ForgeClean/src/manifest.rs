use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CleanupCategory {
    SupersededPackage,
    PackageSignature,
    PartialDownload,
}

impl CleanupCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SupersededPackage => "superseded-package",
            Self::PackageSignature => "package-signature",
            Self::PartialDownload => "partial-download",
        }
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "superseded-package" => Ok(Self::SupersededPackage),
            "package-signature" => Ok(Self::PackageSignature),
            "partial-download" => Ok(Self::PartialDownload),
            _ => Err(format!("unknown cleanup category: {value}")),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileIdentity {
    pub dev: u64,
    pub ino: u64,
    pub size: u64,
    pub mtime_sec: i64,
    pub mtime_nsec: i64,
}

impl FileIdentity {
    pub fn from_metadata(metadata: &fs::Metadata) -> Self {
        Self {
            dev: metadata.dev(),
            ino: metadata.ino(),
            size: metadata.size(),
            mtime_sec: metadata.mtime(),
            mtime_nsec: metadata.mtime_nsec(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CleanupEntry {
    pub path: PathBuf,
    pub bytes: u64,
    pub category: CleanupCategory,
    pub reason: String,
    pub identity: FileIdentity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CleanupBatch {
    pub root: PathBuf,
    pub keep_versions: usize,
    pub partial_age_days: u64,
    pub entries: Vec<CleanupEntry>,
}

impl CleanupBatch {
    pub fn total_bytes(&self) -> u64 {
        self.entries.iter().map(|entry| entry.bytes).sum()
    }

    pub fn write_to(&self, path: &Path) -> Result<(), String> {
        let file = File::create(path)
            .map_err(|e| format!("cannot create manifest {}: {e}", path.display()))?;
        let mut out = BufWriter::new(file);
        writeln!(out, "FORGECLEAN-MANIFEST\t1").map_err(io_err)?;
        writeln!(
            out,
            "ROOT\t{}",
            hex_encode(self.root.as_os_str().as_bytes())
        )
        .map_err(io_err)?;
        writeln!(out, "KEEP\t{}", self.keep_versions).map_err(io_err)?;
        writeln!(out, "PARTIAL_AGE_DAYS\t{}", self.partial_age_days).map_err(io_err)?;
        for entry in &self.entries {
            writeln!(
                out,
                "ENTRY\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                entry.category.as_str(),
                entry.bytes,
                entry.identity.dev,
                entry.identity.ino,
                entry.identity.size,
                entry.identity.mtime_sec,
                entry.identity.mtime_nsec,
                hex_encode(entry.path.as_os_str().as_bytes()),
                hex_encode(entry.reason.as_bytes()),
            )
            .map_err(io_err)?;
        }
        out.flush().map_err(io_err)
    }

    pub fn read_from(path: &Path) -> Result<Self, String> {
        let file = File::open(path)
            .map_err(|e| format!("cannot open manifest {}: {e}", path.display()))?;
        let mut lines = BufReader::new(file).lines();
        let header = lines
            .next()
            .ok_or_else(|| "manifest is empty".to_owned())?
            .map_err(io_err)?;
        if header != "FORGECLEAN-MANIFEST\t1" {
            return Err("unsupported or invalid ForgeClean manifest".to_owned());
        }

        let root_line = next_required(&mut lines, "ROOT")?;
        let keep_line = next_required(&mut lines, "KEEP")?;
        let age_line = next_required(&mut lines, "PARTIAL_AGE_DAYS")?;
        let root = PathBuf::from(std::ffi::OsString::from_vec(hex_decode(&root_line)?));
        let keep_versions = keep_line
            .parse::<usize>()
            .map_err(|e| format!("invalid KEEP value: {e}"))?;
        if keep_versions == 0 {
            return Err("manifest KEEP must be at least 1".to_owned());
        }
        let partial_age_days = age_line
            .parse::<u64>()
            .map_err(|e| format!("invalid PARTIAL_AGE_DAYS value: {e}"))?;

        let mut entries = Vec::new();
        for line in lines {
            let line = line.map_err(io_err)?;
            if line.trim().is_empty() {
                continue;
            }
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() != 10 || parts[0] != "ENTRY" {
                return Err(format!("malformed manifest entry: {line}"));
            }
            let category = CleanupCategory::parse(parts[1])?;
            let bytes = parse_u64(parts[2], "entry bytes")?;
            let identity = FileIdentity {
                dev: parse_u64(parts[3], "device")?,
                ino: parse_u64(parts[4], "inode")?,
                size: parse_u64(parts[5], "identity size")?,
                mtime_sec: parts[6]
                    .parse::<i64>()
                    .map_err(|e| format!("invalid mtime seconds: {e}"))?,
                mtime_nsec: parts[7]
                    .parse::<i64>()
                    .map_err(|e| format!("invalid mtime nanoseconds: {e}"))?,
            };
            let path = PathBuf::from(std::ffi::OsString::from_vec(hex_decode(parts[8])?));
            let reason = String::from_utf8(hex_decode(parts[9])?)
                .map_err(|e| format!("manifest reason is not UTF-8: {e}"))?;
            entries.push(CleanupEntry {
                path,
                bytes,
                category,
                reason,
                identity,
            });
        }

        Ok(Self {
            root,
            keep_versions,
            partial_age_days,
            entries,
        })
    }
}

fn next_required<I>(lines: &mut I, expected: &str) -> Result<String, String>
where
    I: Iterator<Item = Result<String, std::io::Error>>,
{
    let line = lines
        .next()
        .ok_or_else(|| format!("manifest missing {expected}"))?
        .map_err(io_err)?;
    let (key, value) = line
        .split_once('\t')
        .ok_or_else(|| format!("malformed {expected} line"))?;
    if key != expected {
        return Err(format!("expected {expected}, found {key}"));
    }
    Ok(value.to_owned())
}

fn parse_u64(value: &str, name: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|e| format!("invalid {name}: {e}"))
}

fn io_err(error: std::io::Error) -> String {
    error.to_string()
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn hex_decode(text: &str) -> Result<Vec<u8>, String> {
    if !text.len().is_multiple_of(2) {
        return Err("invalid odd-length hex field".to_owned());
    }
    let mut out = Vec::with_capacity(text.len() / 2);
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let high = hex_nibble(bytes[i])?;
        let low = hex_nibble(bytes[i + 1])?;
        out.push((high << 4) | low);
        i += 2;
    }
    Ok(out)
}

fn hex_nibble(byte: u8) -> Result<u8, String> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(format!("invalid hex byte: {byte}")),
    }
}

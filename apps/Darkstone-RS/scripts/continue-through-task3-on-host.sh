#!/usr/bin/env bash
set -euo pipefail

repo="${1:-$(pwd)}"
cd "$repo"

need() {
  command -v "$1" >/dev/null 2>&1 || {
    printf 'DARKSTONE_TASK3_BLOCKED=missing-%s\n' "$1" >&2
    exit 70
  }
}

need cargo
need rustc
need python3

in_git=false
if command -v git >/dev/null 2>&1 && git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  in_git=true
  branch="$(git branch --show-current)"
  if [[ "$branch" != "feature/initial-rebuild" ]]; then
    printf 'DARKSTONE_TASK3_BLOCKED=wrong-branch:%s\n' "$branch" >&2
    exit 71
  fi
else
  printf 'DARKSTONE_TASK3_GIT=standalone-no-commit\n'
fi

printf 'RUSTC=%s\n' "$(rustc --version)"
printf 'CARGO=%s\n' "$(cargo --version)"

# Tasks 1 and 2 must be GREEN first. Never restage Task 2 RED over an
# already-implemented parser; re-verify it instead.
if [[ -f crates/darkstone-mtf/src/reader.rs ]] && grep -q '^pub struct MtfArchive' crates/darkstone-mtf/src/reader.rs; then
  cargo fmt --all --check
  cargo test -p darkstone-assets
  cargo clippy -p darkstone-assets --all-targets -- -D warnings
  cargo test -p darkstone-mtf
  cargo clippy -p darkstone-mtf --all-targets -- -D warnings
  printf 'DARKSTONE_TASK2_GREEN=PASS-existing\n'
else
  ./scripts/continue-through-task2-on-host.sh "$repo"
fi

# Stage only the Task-3 package/test harness before RED verification.
python3 - <<'PY'
from pathlib import Path
p = Path('Cargo.toml')
s = p.read_text()
member = '  "tools/darkstone-compat-report",\n'
if '"tools/darkstone-compat-report"' not in s:
    marker = '  "crates/darkstone-mtf",\n'
    if marker not in s:
        raise SystemExit('DARKSTONE_TASK3_BLOCKED=workspace-members-shape')
    s = s.replace(marker, marker + member)
p.write_text(s)
PY

mkdir -p tools/darkstone-compat-report/tests tools/darkstone-compat-report/src
cat > tools/darkstone-compat-report/Cargo.toml <<'TOML'
[package]
name = "darkstone-compat-report"
version.workspace = true
edition.workspace = true
license.workspace = true
rust-version.workspace = true

[dependencies]
clap.workspace = true
darkstone-assets = { path = "../../crates/darkstone-assets" }
darkstone-mtf = { path = "../../crates/darkstone-mtf" }
serde_json.workspace = true
sha2.workspace = true
thiserror.workspace = true

[[test]]
name = "scan"
path = "tests/scan.rs"
TOML

cat > tools/darkstone-compat-report/tests/scan.rs <<'RUST'
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use darkstone_compat_report::{scan_install, ScanError};

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn synthetic_mtf(name: &[u8], payload: &[u8]) -> Vec<u8> {
    let directory_len = 4 + 4 + name.len() + 4 + 4;
    let mut out = Vec::with_capacity(directory_len + payload.len());
    push_u32(&mut out, 1);
    push_u32(&mut out, name.len() as u32);
    out.extend_from_slice(name);
    push_u32(&mut out, directory_len as u32);
    push_u32(&mut out, payload.len() as u32);
    out.extend_from_slice(payload);
    out
}

struct TestDir(PathBuf);

impl TestDir {
    fn new(label: &str) -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "darkstone-rs-{label}-{}-{stamp}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn scans_archives_read_only_and_hashes_them() {
    let dir = TestDir::new("scan");
    let data = synthetic_mtf(b"MODEL\\TEST.O3D\0", b"o3d-bytes");
    let music = synthetic_mtf(b"MUSIC\\TEST.MP2\0", b"music-bytes");
    let data_path = dir.path().join("DATA.MTF");
    let music_path = dir.path().join("MUSIC.MTF");
    let unrelated_path = dir.path().join("notes.txt");
    fs::write(&data_path, &data).unwrap();
    fs::write(&music_path, &music).unwrap();
    fs::write(&unrelated_path, b"leave me alone").unwrap();

    let before_data = fs::read(&data_path).unwrap();
    let before_music = fs::read(&music_path).unwrap();
    let before_unrelated = fs::read(&unrelated_path).unwrap();
    let before_data_mtime = fs::metadata(&data_path).unwrap().modified().unwrap();
    let before_music_mtime = fs::metadata(&music_path).unwrap().modified().unwrap();
    let before_unrelated_mtime = fs::metadata(&unrelated_path).unwrap().modified().unwrap();

    let report = scan_install(dir.path()).unwrap();

    assert_eq!(report.schema_version, 1);
    assert_eq!(report.install_path, dir.path());
    assert_eq!(report.archives.len(), 2);
    assert!(report.archives.iter().all(|item| item.sha256.len() == 64));
    assert_eq!(report.summary.archives_parsed, 2);
    assert_eq!(report.summary.entries_total, 2);
    assert!(report
        .assets
        .iter()
        .any(|item| item.internal_path == "MODEL\\TEST.O3D"));
    assert!(report
        .assets
        .iter()
        .any(|item| item.internal_path == "MUSIC\\TEST.MP2"));

    assert_eq!(fs::read(&data_path).unwrap(), before_data);
    assert_eq!(fs::read(&music_path).unwrap(), before_music);
    assert_eq!(fs::read(&unrelated_path).unwrap(), before_unrelated);
    assert_eq!(fs::metadata(&data_path).unwrap().modified().unwrap(), before_data_mtime);
    assert_eq!(fs::metadata(&music_path).unwrap().modified().unwrap(), before_music_mtime);
    assert_eq!(
        fs::metadata(&unrelated_path).unwrap().modified().unwrap(),
        before_unrelated_mtime
    );
}

#[test]
fn missing_optional_archives_do_not_fail_scan() {
    let dir = TestDir::new("optional");
    fs::write(
        dir.path().join("DATA.MTF"),
        synthetic_mtf(b"ONLY\\ONE.BIN\0", b"x"),
    )
    .unwrap();

    let report = scan_install(dir.path()).unwrap();

    assert_eq!(report.summary.archives_parsed, 1);
    assert_eq!(report.summary.entries_total, 1);
    assert_eq!(report.archives.len(), 1);
    assert_eq!(report.archives[0].name, "DATA.MTF");
}

#[test]
fn missing_data_archive_is_an_invalid_install() {
    let dir = TestDir::new("missing-data");
    let err = scan_install(dir.path()).unwrap_err();

    assert!(matches!(err, ScanError::MissingRequiredArchive { .. }));
}

#[test]
fn malformed_required_data_archive_is_a_required_parse_error() {
    let dir = TestDir::new("bad-data");
    fs::write(dir.path().join("DATA.MTF"), b"not-an-mtf").unwrap();

    let err = scan_install(dir.path()).unwrap_err();

    assert!(matches!(err, ScanError::RequiredArchiveParse { .. }));
}

#[test]
fn malformed_optional_archive_is_reported_without_aborting() {
    let dir = TestDir::new("bad-optional");
    fs::write(
        dir.path().join("DATA.MTF"),
        synthetic_mtf(b"GOOD.BIN\0", b"ok"),
    )
    .unwrap();
    fs::write(dir.path().join("MUSIC.MTF"), b"not-an-mtf").unwrap();

    let report = scan_install(dir.path()).unwrap();

    assert_eq!(report.summary.archives_parsed, 1);
    assert_eq!(report.summary.errors, 1);
    assert!(report.assets.iter().any(|item| {
        item.archive_name == "MUSIC.MTF"
            && item.internal_path == "<archive>"
            && item.diagnostic.is_some()
    }));
}
RUST

cargo fmt --all

set +e
red_output="$(cargo test -p darkstone-compat-report --test scan 2>&1)"
red_rc=$?
set -e
printf '%s\n' "$red_output"
if (( red_rc == 0 )); then
  echo 'DARKSTONE_TASK3_RED=FAIL-test-passed-before-implementation' >&2
  exit 72
fi
if ! grep -Eq 'unresolved import|unresolved crate|use of unresolved module|no external crate' <<<"$red_output"; then
  echo 'DARKSTONE_TASK3_RED=FAIL-unexpected-failure' >&2
  exit 73
fi
printf 'DARKSTONE_TASK3_RED=PASS\n'

# GREEN: only after RED is observed do we install the scanner library/CLI.
cat > tools/darkstone-compat-report/src/lib.rs <<'RUST'
mod report;
mod scan;

pub use report::write_json_report;
pub use scan::{scan_install, ScanError};
RUST

cat > tools/darkstone-compat-report/src/scan.rs <<'RUST'
use std::{
    fs::File,
    io::{self, Read},
    path::{Path, PathBuf},
    sync::Arc,
};

use darkstone_assets::{
    ArchiveFingerprint, AssetStatus, CompatibilityReport, SupportState,
};
use darkstone_mtf::{Limits, MtfArchive, MtfError};
use sha2::{Digest, Sha256};
use thiserror::Error;

const ARCHIVES: &[(&str, bool)] = &[
    ("DATA.MTF", true),
    ("MUSIC.MTF", false),
    ("VOICES1.MTF", false),
];

#[derive(Debug, Error)]
pub enum ScanError {
    #[error("install path is not a directory: {path}")]
    InvalidInstallPath { path: PathBuf },
    #[error("required archive is missing: {path}")]
    MissingRequiredArchive { path: PathBuf },
    #[error("failed to read {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("required archive {path} could not be parsed safely: {source}")]
    RequiredArchiveParse {
        path: PathBuf,
        #[source]
        source: MtfError,
    },
}

pub fn scan_install(path: &Path) -> Result<CompatibilityReport, ScanError> {
    if !path.is_dir() {
        return Err(ScanError::InvalidInstallPath {
            path: path.to_path_buf(),
        });
    }

    let required = path.join("DATA.MTF");
    if !required.is_file() {
        return Err(ScanError::MissingRequiredArchive { path: required });
    }

    let mut report = CompatibilityReport::new(path.to_path_buf(), Vec::new());

    for &(archive_name, required) in ARCHIVES {
        let archive_path = path.join(archive_name);
        if !archive_path.is_file() {
            continue;
        }

        let (bytes, sha256) = read_and_hash(&archive_path)?;
        let parsed = MtfArchive::parse(Arc::<[u8]>::from(bytes), Limits::default());
        let archive = match parsed {
            Ok(archive) => archive,
            Err(source) if required => {
                return Err(ScanError::RequiredArchiveParse {
                    path: archive_path,
                    source,
                });
            }
            Err(source) => {
                let mut status = AssetStatus::new(
                    archive_name,
                    "<archive>",
                    0,
                    SupportState::Error,
                );
                status.diagnostic = Some(source.to_string());
                report.assets.push(status);
                report.summary.errors += 1;
                continue;
            }
        };

        report.archives.push(ArchiveFingerprint {
            name: archive_name.to_owned(),
            sha256,
        });
        report.summary.archives_parsed += 1;
        report.summary.entries_total += archive.entries().len() as u64;

        for entry in archive.entries() {
            report.assets.push(AssetStatus::new(
                archive_name,
                entry.name.clone(),
                u64::from(entry.uncompressed_size),
                SupportState::Unknown,
            ));
        }
    }

    Ok(report)
}

fn read_and_hash(path: &Path) -> Result<(Vec<u8>, String), ScanError> {
    let mut file = File::open(path).map_err(|source| ScanError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let capacity = file
        .metadata()
        .ok()
        .and_then(|meta| usize::try_from(meta.len()).ok())
        .unwrap_or(0);
    let mut bytes = Vec::with_capacity(capacity);
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];

    loop {
        let count = file.read(&mut buf).map_err(|source| ScanError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        if count == 0 {
            break;
        }
        hasher.update(&buf[..count]);
        bytes.extend_from_slice(&buf[..count]);
    }

    Ok((bytes, format!("{:x}", hasher.finalize())))
}
RUST

cat > tools/darkstone-compat-report/src/report.rs <<'RUST'
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

pub fn write_json_report(report: &CompatibilityReport, path: &Path) -> Result<(), ReportWriteError> {
    let file = File::create(path).map_err(|source| ReportWriteError::Create {
        path: path.display().to_string(),
        source,
    })?;
    serde_json::to_writer_pretty(file, report)?;
    Ok(())
}
RUST

cat > tools/darkstone-compat-report/src/main.rs <<'RUST'
use std::{path::PathBuf, process::ExitCode};

use clap::Parser;
use darkstone_compat_report::{scan_install, write_json_report, ScanError};

#[derive(Debug, Parser)]
#[command(name = "darkstone-compat-report")]
#[command(about = "Read-only Darkstone installation compatibility scanner")]
struct Cli {
    #[arg(long)]
    install: PathBuf,
    #[arg(long)]
    output: Option<PathBuf>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let report = match scan_install(&cli.install) {
        Ok(report) => report,
        Err(ScanError::RequiredArchiveParse { .. }) => {
            eprintln!("required DATA.MTF could not be parsed safely");
            return ExitCode::from(3);
        }
        Err(err) => {
            eprintln!("{err}");
            return ExitCode::from(2);
        }
    };

    if let Some(path) = cli.output {
        if let Err(err) = write_json_report(&report, &path) {
            eprintln!("{err}");
            return ExitCode::from(2);
        }
    } else {
        match serde_json::to_string_pretty(&report) {
            Ok(json) => println!("{json}"),
            Err(err) => {
                eprintln!("failed to serialize report: {err}");
                return ExitCode::from(2);
            }
        }
    }

    ExitCode::SUCCESS
}
RUST

cargo fmt --all
cargo fmt --all --check
cargo test -p darkstone-compat-report
cargo clippy -p darkstone-compat-report --all-targets -- -D warnings

# Manual synthetic CLI verification. The files are generated and removed here;
# no DATA.MTF payload is committed to the repository.
fixture_dir="tests/fixtures/fake-install"
rm -rf "$fixture_dir"
mkdir -p "$fixture_dir"
python3 - <<'PY'
from pathlib import Path

def u32(v):
    return int(v).to_bytes(4, 'little')

name = b'SYNTHETIC\\HELLO.BIN\0'
payload = b'hello'
offset = 4 + 4 + len(name) + 4 + 4
blob = b''.join([u32(1), u32(len(name)), name, u32(offset), u32(len(payload)), payload])
Path('tests/fixtures/fake-install/DATA.MTF').write_bytes(blob)
PY

report_path="$(mktemp -t darkstone-report.XXXXXX.json)"
trap 'rm -rf "$fixture_dir"; rm -f "$report_path"' EXIT
cargo run -p darkstone-compat-report -- --install "$fixture_dir" --output "$report_path"
python3 -m json.tool "$report_path" >/dev/null
python3 - "$report_path" <<'PY'
import json, sys
p = sys.argv[1]
with open(p, 'r', encoding='utf-8') as f:
    data = json.load(f)
assert data['schema_version'] == 1
assert data['summary']['archives_parsed'] == 1
assert data['summary']['entries_total'] == 1
assert data['archives'][0]['name'] == 'DATA.MTF'
print('DARKSTONE_TASK3_REPORT=PASS')
PY
rm -rf "$fixture_dir"
rm -f "$report_path"
trap - EXIT

printf 'DARKSTONE_TASK3_GREEN=PASS\n'

if [[ "$in_git" == true ]]; then
  git add Cargo.toml tools/darkstone-compat-report scripts/continue-through-task3-on-host.sh
  if ! git diff --cached --quiet; then
    git commit -m 'feat: add read-only Darkstone install scanner'
  fi
  printf 'DARKSTONE_TASK3_COMMIT=%s\n' "$(git rev-parse --short HEAD)"
else
  printf 'DARKSTONE_TASK3_COMMIT=SKIPPED-standalone\n'
fi

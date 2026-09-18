use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};

const RECORDING_NAME: &str = "mic-test.wav";

pub fn recording_path() -> io::Result<PathBuf> {
    let home = env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "HOME is not set"))?;
    Ok(home
        .join(".cache/aetherforge-beacn-control")
        .join(RECORDING_NAME))
}

pub fn start_10s(target: &str) -> io::Result<PathBuf> {
    let path = recording_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut command = Command::new("timeout");
    command.args(record_arguments(target, &path));
    spawn_reaped(command)?;
    Ok(path)
}

pub fn play(target: &str) -> io::Result<PathBuf> {
    let path = recording_path()?;
    if !path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "record a mic test first",
        ));
    }

    let mut command = Command::new("pw-play");
    command.arg("--target").arg(target).arg(&path);
    spawn_reaped(command)?;
    Ok(path)
}

pub fn record_arguments(target: &str, path: &Path) -> Vec<String> {
    vec![
        "10s".to_owned(),
        "pw-record".to_owned(),
        "--target".to_owned(),
        target.to_owned(),
        path.display().to_string(),
    ]
}

fn spawn_reaped(mut command: Command) -> io::Result<()> {
    let child = command.spawn()?;
    let _reaper = std::thread::spawn(move || reap(child));
    Ok(())
}

fn reap(mut child: Child) {
    let _ = child.wait();
}

extern crate forgeclean as forgeclean_lib;

use forgeclean_lib::project_routing::{
    ProjectIndex, canonical_project_root, has_project_marker, project_class_for_name,
    project_root_is_trusted,
};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone)]
struct SeenState {
    len: u64,
    modified: Option<SystemTime>,
    first_stable: SystemTime,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("FORGECLEAN_PROJECT_ROUTER=FAIL:{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let home = PathBuf::from(env::var_os("HOME").ok_or("HOME is not set")?);
    let downloads = home.join("Downloads");
    fs::create_dir_all(&downloads)?;
    ensure_project_files_root(&downloads)?;

    let stable_seconds = arg_u64(&args, "--stable-seconds", 30);
    let poll_seconds = arg_u64(&args, "--poll-seconds", 2).max(1);
    let migrate_legacy = args.iter().any(|arg| arg == "--migrate-legacy");
    let watch = args.iter().any(|arg| arg == "--watch");
    let exclude_prefix = arg_string(&args, "--exclude-prefix");

    if migrate_legacy {
        collapse_legacy_project_folders(&downloads)?;
        migrate_legacy_general_buckets(&downloads)?;
    }

    if !watch {
        let moved = route_once(&downloads, stable_seconds, None, exclude_prefix.as_deref())?;
        println!("FORGECLEAN_PROJECT_ROUTER=PASS");
        println!("MOVED={moved}");
        println!("GENERAL_POLICY=LEAVE_IN_DOWNLOADS");
        println!("PROJECT_ROOT={}", downloads.join("Project Files").display());
        return Ok(());
    }

    let mut seen = HashMap::<PathBuf, SeenState>::new();
    println!("FORGECLEAN_PROJECT_ROUTER=WATCHING");
    println!("GENERAL_POLICY=LEAVE_IN_DOWNLOADS");
    println!("PROJECT_ROOT={}", downloads.join("Project Files").display());
    loop {
        match route_once(
            &downloads,
            stable_seconds,
            Some(&mut seen),
            exclude_prefix.as_deref(),
        ) {
            Ok(moved) if moved > 0 => println!("FORGECLEAN_PROJECT_ROUTER_MOVED={moved}"),
            Ok(_) => {}
            Err(error) => eprintln!("FORGECLEAN_PROJECT_ROUTER_CYCLE_ERROR={error}"),
        }
        thread::sleep(Duration::from_secs(poll_seconds));
    }
}

fn route_once(
    downloads: &Path,
    stable_seconds: u64,
    mut seen: Option<&mut HashMap<PathBuf, SeenState>>,
    exclude_prefix: Option<&str>,
) -> Result<usize, Box<dyn std::error::Error>> {
    let index = ProjectIndex::discover(downloads);
    let mut moved = 0usize;
    let now = SystemTime::now();
    let entries = fs::read_dir(downloads)?;
    for entry in entries.flatten() {
        let path = entry.path();
        let name = match path.file_name().and_then(|value| value.to_str()) {
            Some(name) => name.to_owned(),
            None => continue,
        };
        if matches!(name.as_str(), "ForgeClean" | "Project Files")
            || exclude_prefix.is_some_and(|prefix| name.starts_with(prefix))
        {
            continue;
        }
        let meta = match fs::symlink_metadata(&path) {
            Ok(meta) => meta,
            Err(_) => continue,
        };
        if meta.file_type().is_symlink() {
            continue;
        }

        if let Some(states) = seen.as_deref_mut() {
            let modified = meta.modified().ok();
            let current = SeenState {
                len: meta.len(),
                modified,
                first_stable: now,
            };
            match states.get_mut(&path) {
                Some(previous)
                    if previous.len == current.len && previous.modified == current.modified =>
                {
                    if now
                        .duration_since(previous.first_stable)
                        .unwrap_or_default()
                        < Duration::from_secs(stable_seconds)
                    {
                        continue;
                    }
                }
                Some(previous) => {
                    *previous = current;
                    continue;
                }
                None => {
                    states.insert(path.clone(), current);
                    continue;
                }
            }
        } else if stable_seconds > 0 {
            let modified = meta.modified().unwrap_or(now);
            if now.duration_since(modified).unwrap_or_default()
                < Duration::from_secs(stable_seconds)
            {
                continue;
            }
        }

        let Some(family) = index.family_for_top_level(&path) else {
            continue;
        };
        if route_path(downloads, &path, &family)? {
            if let Some(states) = seen.as_deref_mut() {
                states.remove(&path);
            }
            moved += 1;
        }
    }
    Ok(moved)
}

fn route_path(
    downloads: &Path,
    source: &Path,
    family: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    let meta = fs::symlink_metadata(source)?;
    if meta.file_type().is_symlink() {
        return Ok(false);
    }
    let name = source
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or("non-UTF8 project artifact name")?;
    let source_tree = meta.is_dir() && has_project_marker(source);
    let class = project_class_for_name(name, source_tree);
    let project_root = canonical_project_root(downloads, family);
    let bucket = match class {
        "SOURCE" => project_root.join("Active"),
        "RELEASES" => project_root.join("Releases"),
        "EVIDENCE" => project_root.join("Downloads/Evidence"),
        _ => project_root.join("Downloads/Files"),
    };
    fs::create_dir_all(&bucket)?;
    let destination = unique_destination(&bucket, name);
    move_same_filesystem(source, &destination)?;
    if source_tree {
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let _ = symlink(&destination, source);
        }
    }
    println!(
        "FORGECLEAN_PROJECT_ROUTE=PROJECT:{}\tCLASS:{}\tFROM:{}\tTO:{}",
        escape(family),
        class,
        escape(&source.display().to_string()),
        escape(&destination.display().to_string())
    );
    Ok(true)
}

fn collapse_legacy_project_folders(downloads: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let project_files = downloads.join("Project Files");
    let index = ProjectIndex::discover(downloads);
    let Ok(entries) = fs::read_dir(&project_files) else {
        return Ok(());
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let meta = match fs::symlink_metadata(&path) {
            Ok(meta) => meta,
            Err(_) => continue,
        };
        if meta.file_type().is_symlink() || !meta.is_dir() || project_root_is_trusted(&path) {
            continue;
        }
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if let Some(family) = index.family_for_artifact_name(name) {
            let target_root = canonical_project_root(downloads, &family);
            if target_root != path {
                let legacy_bucket = target_root.join("Downloads/Legacy");
                fs::create_dir_all(&legacy_bucket)?;
                let destination = unique_destination(&legacy_bucket, name);
                move_same_filesystem(&path, &destination)?;
                println!(
                    "FORGECLEAN_PROJECT_COLLAPSE=PROJECT:{}\tFROM:{}\tTO:{}",
                    escape(&family),
                    escape(&path.display().to_string()),
                    escape(&destination.display().to_string())
                );
            }
        } else {
            let unassigned = downloads.join("Unassigned Project Artifacts");
            fs::create_dir_all(&unassigned)?;
            let destination = unique_destination(&unassigned, name);
            move_same_filesystem(&path, &destination)?;
            println!(
                "FORGECLEAN_PROJECT_COLLAPSE=UNASSIGNED\tFROM:{}\tTO:{}",
                escape(&path.display().to_string()),
                escape(&destination.display().to_string())
            );
        }
    }
    Ok(())
}

fn migrate_legacy_general_buckets(downloads: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let index = ProjectIndex::discover(downloads);
    let forgeclean = downloads.join("ForgeClean");
    for category in [
        "Packages",
        "Documents",
        "Images",
        "Video",
        "Audio",
        "Archives",
        "Logs",
        "Installers",
        "Temporary",
        "Other",
    ] {
        let root = forgeclean.join(category);
        let Ok(entries) = fs::read_dir(&root) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let meta = match fs::symlink_metadata(&path) {
                Ok(meta) => meta,
                Err(_) => continue,
            };
            if meta.file_type().is_symlink() {
                continue;
            }
            if let Some(family) = index.family_for_top_level(&path) {
                route_path(downloads, &path, &family)?;
                continue;
            }
            let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            let destination = unique_destination(downloads, name);
            move_same_filesystem(&path, &destination)?;
            println!(
                "FORGECLEAN_GENERAL_RESTORE=FROM:{}\tTO:{}",
                escape(&path.display().to_string()),
                escape(&destination.display().to_string())
            );
        }
    }
    Ok(())
}

fn ensure_project_files_root(downloads: &Path) -> io::Result<()> {
    let project_files = downloads.join("Project Files");
    let legacy = downloads.join("ForgeClean/Projects");
    fs::create_dir_all(downloads.join("ForgeClean"))?;

    let project_meta = fs::symlink_metadata(&project_files).ok();
    let legacy_meta = fs::symlink_metadata(&legacy).ok();

    if project_meta.is_none() {
        if legacy_meta
            .as_ref()
            .is_some_and(|meta| meta.is_dir() && !meta.file_type().is_symlink())
        {
            fs::rename(&legacy, &project_files)?;
        } else {
            fs::create_dir_all(&project_files)?;
        }
    }

    if let Ok(meta) = fs::symlink_metadata(&legacy) {
        if meta.file_type().is_symlink() {
            return Ok(());
        }
        if meta.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!(
                    "both project roots exist; refusing automatic merge: {} and {}",
                    legacy.display(),
                    project_files.display()
                ),
            ));
        }
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        symlink(&project_files, &legacy)?;
    }
    Ok(())
}

fn move_same_filesystem(source: &Path, destination: &Path) -> io::Result<()> {
    match fs::rename(source, destination) {
        Ok(()) => Ok(()),
        Err(error) if error.raw_os_error() == Some(18) => Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!(
                "cross-filesystem move refused for safety: {} -> {}",
                source.display(),
                destination.display()
            ),
        )),
        Err(error) => Err(error),
    }
}

fn unique_destination(parent: &Path, name: &str) -> PathBuf {
    let first = parent.join(name);
    if !first.exists() && fs::symlink_metadata(&first).is_err() {
        return first;
    }
    for index in 2..=10_000u32 {
        let candidate = parent.join(format!("{name}.duplicate-{index}"));
        if !candidate.exists() && fs::symlink_metadata(&candidate).is_err() {
            return candidate;
        }
    }
    parent.join(format!("{name}.duplicate-{}", std::process::id()))
}

fn arg_string(args: &[String], flag: &str) -> Option<String> {
    args.windows(2)
        .find(|pair| pair[0] == flag)
        .and_then(|pair| pair.get(1))
        .cloned()
}

fn arg_u64(args: &[String], flag: &str, default: u64) -> u64 {
    args.windows(2)
        .find(|pair| pair[0] == flag)
        .and_then(|pair| pair[1].parse::<u64>().ok())
        .unwrap_or(default)
}

fn escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

use forgeclean::coldpack_gc::{
    ColdPackStoreReport, coldpack_audit, coldpack_gc_apply, coldpack_gc_preview, coldpack_status,
};
use forgeclean::coldstore::{archive_to_coldpack, restore_cold_archive_with_store};
use forgeclean::gui_state::{GuiSettings, load_settings};
use forgeclean::manifest::CleanupBatch;
use forgeclean::offload::{
    OffloadReport, offload_batch, partition_external_batch, prepare_destination_under_mount,
};
use forgeclean::organizer::{ForgeLayout, classify_download, organize_once_with_alias_policy};
use forgeclean::package::{ArchVersionComparator, installed_versions_from_pacman};
use forgeclean::purge::{purge_batch, verify_batch};
use forgeclean::registry::{ProjectRegistry, create_legacy_alias};
use forgeclean::scan::{ScanOptions, protect_installed_versions, scan_cache};
use forgeclean::storage::{AutoMode, detect_external_drives};
use forgeclean::system_scan::{
    SYSTEM_PACMAN_CACHE, default_offload_report_path, default_system_manifest_path,
    system_scan_options,
};
use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const VERSION: &str = "1.0.1";
const DEFAULT_MANIFEST: &str = "ForgeClean-v1.0.1-BATCH.txt";

fn main() -> ExitCode {
    match run(env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("FORGECLEAN=FAIL");
            eprintln!("ERROR={e}");
            ExitCode::from(1)
        }
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    let Some(command) = args.first().map(String::as_str) else {
        print_help();
        return Ok(());
    };
    match command {
        "scan" => command_scan(&args[1..]),
        "scan-system" => command_scan_system(&args[1..]),
        "storage-status" => command_storage_status(&args[1..]),
        "auto" => command_auto(&args[1..]),
        "verify" => command_verify(&args[1..]),
        "purge" => command_purge(&args[1..]),
        "organize-once" => command_organize_once(&args[1..]),
        "watch" => command_watch(&args[1..]),
        "activate" => command_activate(&args[1..]),
        "resolve-project" => command_resolve_project(&args[1..]),
        "build" => command_build(&args[1..]),
        "archive" => command_archive(&args[1..]),
        "restore" => command_restore(&args[1..]),
        "coldpack-status" => command_coldpack_status(&args[1..]),
        "coldpack-audit" => command_coldpack_audit(&args[1..]),
        "coldpack-gc" => command_coldpack_gc(&args[1..]),
        "--version" | "version" => {
            println!("ForgeClean v{VERSION}");
            Ok(())
        }
        "--help" | "-h" | "help" => {
            print_help();
            Ok(())
        }
        other => Err(format!("unknown command: {other}")),
    }
}

fn command_scan(args: &[String]) -> Result<(), String> {
    let mut options = ScanOptions::default();
    let mut manifest = PathBuf::from(DEFAULT_MANIFEST);
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--cache-dir" => {
                i += 1;
                options.cache_dir = PathBuf::from(value(args, i, "--cache-dir")?);
            }
            "--keep" => {
                i += 1;
                options.keep_versions = value(args, i, "--keep")?
                    .parse()
                    .map_err(|e| format!("invalid --keep: {e}"))?;
            }
            "--partial-age-days" => {
                i += 1;
                options.partial_age_days = value(args, i, "--partial-age-days")?
                    .parse()
                    .map_err(|e| format!("invalid --partial-age-days: {e}"))?;
            }
            "--manifest" => {
                i += 1;
                manifest = PathBuf::from(value(args, i, "--manifest")?);
            }
            flag => return Err(format!("unknown scan option: {flag}")),
        }
        i += 1;
    }
    execute_scan(options, manifest, false)
}

fn command_scan_system(args: &[String]) -> Result<(), String> {
    let (keep_versions, partial_age_days, yes) = parse_auto_options(args, "scan-system", false)?;
    if yes {
        return Err("scan-system never accepts --yes; it is preview-only".to_owned());
    }
    let home = home_dir()?;
    let manifest = default_system_manifest_path(&home);
    ensure_downloads_parent(&manifest)?;
    let options = system_scan_options(keep_versions, partial_age_days)?;
    execute_scan(options, manifest, true)
}

fn command_storage_status(args: &[String]) -> Result<(), String> {
    if !args.is_empty() {
        return Err(format!("unknown storage-status option: {}", args[0]));
    }
    let drives = detect_external_drives(Path::new(SYSTEM_PACMAN_CACHE))?;
    println!("FORGECLEAN_STORAGE_SCAN=PASS");
    println!("EXTERNAL_DRIVES={}", drives.len());
    if let Some(drive) = drives.first() {
        println!("FORGECLEAN_MODE=AUTO_OFFLOAD");
        println!("EXTERNAL_STORAGE=DETECTED");
        println!("EXTERNAL_MOUNT={}", drive.mount_point.display());
        println!("EXTERNAL_DEVICE={}", drive.device_path.display());
        println!(
            "EXTERNAL_UUID={}",
            drive.uuid.as_deref().unwrap_or("UNKNOWN")
        );
        println!(
            "EXTERNAL_TRANSPORT={}",
            drive.transport.as_deref().unwrap_or("UNKNOWN")
        );
    } else {
        println!("FORGECLEAN_MODE=AUTO_CLEAN");
        println!("EXTERNAL_STORAGE=NONE");
    }
    Ok(())
}

fn command_auto(args: &[String]) -> Result<(), String> {
    let (keep_versions, partial_age_days, yes) = parse_auto_options(args, "auto", true)?;
    if !yes {
        return Err("auto requires explicit --yes; nothing was deleted or offloaded".to_owned());
    }

    let home = home_dir()?;
    let manifest = default_system_manifest_path(&home);
    ensure_downloads_parent(&manifest)?;
    let options = system_scan_options(keep_versions, partial_age_days)?;
    let mut batch = scan_cache(&options, &ArchVersionComparator)?;
    let installed = installed_versions_from_pacman()?;
    let pinned = protect_installed_versions(&mut batch, &installed);
    batch.write_to(&manifest)?;

    let preflight = verify_batch(&batch);
    if !preflight.is_ok() {
        for issue in preflight.issues {
            eprintln!("ISSUE={issue}");
        }
        return Err("automatic preflight failed; no source files were removed".to_owned());
    }

    println!("FORGECLEAN_AUTO_SCAN=PASS");
    println!("CACHE_ROOT={}", batch.root.display());
    println!("KEEP_VERSIONS={}", batch.keep_versions);
    println!("PARTIAL_AGE_DAYS={}", batch.partial_age_days);
    println!("INSTALLED_VERSION_PINS={pinned}");
    println!("CANDIDATE_FILES={}", batch.entries.len());
    println!("RECOVERABLE_BYTES={}", batch.total_bytes());
    println!("MANIFEST={}", manifest.display());

    let drives = detect_external_drives(&batch.root)?;
    let mode = drives
        .into_iter()
        .next()
        .map(AutoMode::Offload)
        .unwrap_or(AutoMode::CleanOnly);

    match mode {
        AutoMode::CleanOnly => {
            println!("FORGECLEAN_MODE=AUTO_CLEAN");
            println!("EXTERNAL_STORAGE=NONE");
            let report = purge_batch(&batch);
            println!("DELETED_FILES={}", report.deleted_files);
            println!("DELETED_BYTES={}", report.deleted_bytes);
            println!("SKIPPED_FILES={}", report.skipped.len());
            for skipped in &report.skipped {
                println!("SKIPPED={skipped}");
            }
            if report.is_ok() {
                println!("FORGECLEAN_AUTO_CLEAN=PASS");
                Ok(())
            } else {
                Err("automatic cleanup completed with skipped files; review output".to_owned())
            }
        }
        AutoMode::Offload(drive) => {
            println!("FORGECLEAN_MODE=AUTO_OFFLOAD");
            println!("EXTERNAL_STORAGE=DETECTED");
            println!("EXTERNAL_MOUNT={}", drive.mount_point.display());
            println!("EXTERNAL_DEVICE={}", drive.device_path.display());
            println!(
                "EXTERNAL_UUID={}",
                drive.uuid.as_deref().unwrap_or("UNKNOWN")
            );
            let (offload_batch_data, cleanup_batch_data) = partition_external_batch(&batch);
            let destination = prepare_destination_under_mount(&drive.mount_point)?;
            println!("OFFLOAD_DESTINATION={}", destination.display());
            let report = offload_batch(&offload_batch_data, &destination);
            let cleanup_report = purge_batch(&cleanup_batch_data);
            let report_path = default_offload_report_path(&home);
            write_offload_report(&report_path, &drive.mount_point, &report)?;
            println!("OFFLOADED_FILES={}", report.offloaded_files);
            println!("OFFLOADED_BYTES={}", report.offloaded_bytes);
            println!("OFFLOAD_FAILURES={}", report.failures.len());
            println!("CLEANED_JUNK_FILES={}", cleanup_report.deleted_files);
            println!("CLEANED_JUNK_BYTES={}", cleanup_report.deleted_bytes);
            println!("CLEANUP_FAILURES={}", cleanup_report.skipped.len());
            println!("OFFLOAD_REPORT={}", report_path.display());
            for failure in &report.failures {
                println!("OFFLOAD_FAILURE={failure}");
            }
            for skipped in &cleanup_report.skipped {
                println!("CLEANUP_FAILURE={skipped}");
            }
            if report.is_ok() && cleanup_report.is_ok() {
                println!("FORGECLEAN_AUTO_OFFLOAD=PASS");
                Ok(())
            } else {
                Err(
                    "automatic offload/cleanup had failures; offload-failed source files were preserved"
                        .to_owned(),
                )
            }
        }
    }
}

fn execute_scan(options: ScanOptions, manifest: PathBuf, system_scan: bool) -> Result<(), String> {
    let mut batch = scan_cache(&options, &ArchVersionComparator)?;
    let installed = installed_versions_from_pacman()?;
    let pinned = protect_installed_versions(&mut batch, &installed);
    batch.write_to(&manifest)?;
    println!("FORGECLEAN_SCAN=PASS");
    println!("SYSTEM_SCAN={}", if system_scan { 1 } else { 0 });
    println!("CACHE_ROOT={}", batch.root.display());
    println!("KEEP_VERSIONS={}", batch.keep_versions);
    println!("PARTIAL_AGE_DAYS={}", batch.partial_age_days);
    println!("INSTALLED_VERSION_PINS={pinned}");
    println!("CANDIDATE_FILES={}", batch.entries.len());
    println!("RECOVERABLE_BYTES={}", batch.total_bytes());
    println!("MANIFEST={}", manifest.display());
    for entry in &batch.entries {
        println!(
            "CANDIDATE={}\t{}\t{} bytes\t{}",
            entry.category.as_str(),
            entry.path.display(),
            entry.bytes,
            entry.reason
        );
    }
    println!("PURGE_NOT_EXECUTED=1");
    Ok(())
}

fn command_verify(args: &[String]) -> Result<(), String> {
    let manifest = required_manifest(args)?;
    let batch = CleanupBatch::read_from(&manifest)?;
    let report = verify_batch(&batch);
    println!("VALID_FILES={}", report.valid);
    println!("ISSUES={}", report.issues.len());
    for issue in &report.issues {
        println!("ISSUE={issue}");
    }
    if report.is_ok() {
        println!("FORGECLEAN_VERIFY=PASS");
        Ok(())
    } else {
        Err("manifest verification failed; no files were deleted".to_owned())
    }
}

fn command_purge(args: &[String]) -> Result<(), String> {
    let manifest = required_manifest(args)?;
    if !args.iter().any(|arg| arg == "--yes") {
        return Err("purge requires explicit --yes; nothing was deleted".to_owned());
    }
    let batch = CleanupBatch::read_from(&manifest)?;
    let preflight = verify_batch(&batch);
    if !preflight.is_ok() {
        for issue in preflight.issues {
            eprintln!("ISSUE={issue}");
        }
        return Err("preflight verification failed; purge aborted before deletion".to_owned());
    }
    let report = purge_batch(&batch);
    println!("DELETED_FILES={}", report.deleted_files);
    println!("DELETED_BYTES={}", report.deleted_bytes);
    println!("SKIPPED_FILES={}", report.skipped.len());
    for skipped in &report.skipped {
        println!("SKIPPED={skipped}");
    }
    if report.is_ok() {
        println!("FORGECLEAN_PURGE=PASS");
        Ok(())
    } else {
        Err("purge completed with skipped files; review output".to_owned())
    }
}

fn command_organize_once(args: &[String]) -> Result<(), String> {
    let (downloads, stable_seconds, _poll_seconds, settings) =
        parse_organizer_options(args, false)?;
    let layout = ForgeLayout::new(downloads);
    let report = organize_once_with_alias_policy(
        &layout,
        Duration::from_secs(stable_seconds),
        settings.compatibility_symlinks,
    );
    println!("FORGECLEAN_ORGANIZER=PASS");
    println!("ORGANIZER_ROOT={}", layout.root.display());
    println!("ACTIVATED_PROJECTS={}", report.activated_projects);
    println!("SORTED_ENTRIES={}", report.sorted_entries);
    println!("COLD_ARCHIVES={}", report.cold_archives);
    println!("SKIPPED_ENTRIES={}", report.skipped_entries);
    println!("ISSUES={}", report.issues.len());
    for action in &report.actions {
        println!("ACTION={action}");
    }
    for issue in &report.issues {
        eprintln!("ISSUE={issue}");
    }
    if report.is_ok() {
        Ok(())
    } else {
        Err("organizer completed with issues".to_owned())
    }
}

fn command_watch(args: &[String]) -> Result<(), String> {
    let (downloads, stable_seconds, poll_seconds, settings) = parse_organizer_options(args, true)?;
    let layout = ForgeLayout::new(downloads);
    layout.ensure()?;
    let maintenance_seconds = settings
        .coldpack_maintenance_hours
        .saturating_mul(3_600)
        .max(3_600);
    println!("FORGECLEAN_WATCH=ACTIVE");
    println!("ORGANIZER_ROOT={}", layout.root.display());
    println!("STABLE_SECONDS={stable_seconds}");
    println!("POLL_SECONDS={poll_seconds}");
    println!("COMPATIBILITY_SYMLINKS={}", settings.compatibility_symlinks);
    println!(
        "FORGECLEAN_COLDPACK_MAINTENANCE={}",
        if settings.coldpack_maintenance_enabled {
            "ACTIVE"
        } else {
            "DISABLED"
        }
    );
    if settings.coldpack_maintenance_enabled {
        run_coldpack_maintenance(&layout);
    }
    let mut last_maintenance = Instant::now();
    loop {
        let report = organize_once_with_alias_policy(
            &layout,
            Duration::from_secs(stable_seconds),
            settings.compatibility_symlinks,
        );
        for action in report.actions {
            println!("ACTION={action}");
        }
        for issue in report.issues {
            eprintln!("FORGECLEAN_WATCH_ISSUE={issue}");
        }
        if settings.coldpack_maintenance_enabled
            && last_maintenance.elapsed() >= Duration::from_secs(maintenance_seconds)
        {
            run_coldpack_maintenance(&layout);
            last_maintenance = Instant::now();
        }
        thread::sleep(Duration::from_secs(poll_seconds));
    }
}

fn run_coldpack_maintenance(layout: &ForgeLayout) {
    match unix_now_secs()
        .and_then(|now| coldpack_gc_apply(&layout.root, &layout.coldpack_store(), now))
    {
        Ok(result) => {
            println!("FORGECLEAN_COLDPACK_MAINTENANCE=PASS");
            println!("GC_QUARANTINED_OBJECTS={}", result.quarantined_objects);
            println!("GC_PURGED_OBJECTS={}", result.purged_objects);
        }
        Err(error) => eprintln!("FORGECLEAN_COLDPACK_MAINTENANCE_ISSUE={error}"),
    }
}

fn command_activate(args: &[String]) -> Result<(), String> {
    let project = args
        .first()
        .ok_or_else(|| "activate requires PROJECT".to_owned())?
        .clone();
    let mut path = None;
    let mut legacy_paths = Vec::new();
    let mut downloads = default_downloads_dir()?;
    let mut i = 1usize;
    while i < args.len() {
        match args[i].as_str() {
            "--path" => {
                i += 1;
                path = Some(PathBuf::from(value(args, i, "--path")?));
            }
            "--legacy" => {
                i += 1;
                legacy_paths.push(PathBuf::from(value(args, i, "--legacy")?));
            }
            "--downloads" => {
                i += 1;
                downloads = PathBuf::from(value(args, i, "--downloads")?);
            }
            flag => return Err(format!("unknown activate option: {flag}")),
        }
        i += 1;
    }
    let layout = ForgeLayout::new(&downloads);
    layout.ensure()?;
    let active = path.ok_or_else(|| "activate requires --path PATH".to_owned())?;
    let active = std::fs::canonicalize(&active).map_err(|e| {
        format!(
            "cannot canonicalize active project {}: {e}",
            active.display()
        )
    })?;
    let canonical_root = std::fs::canonicalize(&layout.root).map_err(|e| {
        format!(
            "cannot canonicalize ForgeClean root {}: {e}",
            layout.root.display()
        )
    })?;
    if !active.starts_with(canonical_root.join("Projects")) {
        return Err(format!(
            "active project must live under ForgeClean/Projects: {}",
            active.display()
        ));
    }
    let mut registry = ProjectRegistry::read(&layout.registry_path())?;
    for legacy in &legacy_paths {
        create_legacy_alias(&downloads, legacy, &active)?;
    }
    registry.set_active(&project, active.clone(), legacy_paths);
    registry.write(&layout.registry_path())?;
    println!("FORGECLEAN_ACTIVATE=PASS");
    println!("PROJECT={project}");
    println!("ACTIVE_PATH={}", active.display());
    Ok(())
}

fn command_resolve_project(args: &[String]) -> Result<(), String> {
    let project = args
        .first()
        .ok_or_else(|| "resolve-project requires PROJECT".to_owned())?;
    let downloads = parse_downloads_only(&args[1..], "resolve-project")?;
    let layout = ForgeLayout::new(downloads);
    let registry = ProjectRegistry::read(&layout.registry_path())?;
    let record = registry
        .resolve(project)
        .ok_or_else(|| format!("project is not registered: {project}"))?;
    if !record.active_path.is_dir() {
        return Err(format!(
            "registered Active path is missing: {}",
            record.active_path.display()
        ));
    }
    println!("FORGECLEAN_RESOLVE_PROJECT=PASS");
    println!("PROJECT={}", record.name);
    println!("ACTIVE_PATH={}", record.active_path.display());
    Ok(())
}

fn command_build(args: &[String]) -> Result<(), String> {
    let project = args
        .first()
        .ok_or_else(|| "build requires PROJECT".to_owned())?;
    let separator = args
        .iter()
        .position(|arg| arg == "--")
        .ok_or_else(|| "build requires '-- COMMAND [ARGS...]' after project/options".to_owned())?;
    if separator + 1 >= args.len() {
        return Err("build requires a command after --".to_owned());
    }
    let downloads = parse_downloads_only(&args[1..separator], "build")?;
    let layout = ForgeLayout::new(downloads);
    let registry = ProjectRegistry::read(&layout.registry_path())?;
    let record = registry
        .resolve(project)
        .ok_or_else(|| format!("project is not registered: {project}"))?;
    if !record.active_path.is_dir() {
        return Err(format!(
            "registered Active path is missing: {}",
            record.active_path.display()
        ));
    }
    let command = &args[separator + 1..];
    println!("FORGECLEAN_BUILD_REDIRECT=PASS");
    println!("PROJECT={}", record.name);
    println!("BUILD_CWD={}", record.active_path.display());
    let status = Command::new(&command[0])
        .args(&command[1..])
        .current_dir(&record.active_path)
        .status()
        .map_err(|e| format!("cannot start build command {}: {e}", command[0]))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("build command exited with status {status}"))
    }
}

fn command_archive(args: &[String]) -> Result<(), String> {
    let mut source = None;
    let mut project = None;
    let mut downloads = default_downloads_dir()?;
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--path" => {
                i += 1;
                source = Some(PathBuf::from(value(args, i, "--path")?));
            }
            "--project" => {
                i += 1;
                project = Some(value(args, i, "--project")?.to_owned());
            }
            "--downloads" => {
                i += 1;
                downloads = PathBuf::from(value(args, i, "--downloads")?);
            }
            flag => return Err(format!("unknown archive option: {flag}")),
        }
        i += 1;
    }
    let source = source.ok_or_else(|| "archive requires --path PATH".to_owned())?;
    let layout = ForgeLayout::new(downloads);
    layout.ensure()?;
    refuse_active_project_archive(&layout, &source)?;
    let classified = classify_download(&source);
    let cold_dir = project
        .as_deref()
        .map(|name| layout.project_cold(name))
        .unwrap_or_else(|| layout.category_cold(classified.category));
    let label = source
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("archive");
    let record = archive_to_coldpack(&source, &cold_dir, &layout.coldpack_store(), label)?;
    println!("FORGECLEAN_ARCHIVE=PASS");
    println!("ARCHIVE={}", record.archive_path.display());
    println!("ARCHIVE_SHA256={}", record.archive_sha256);
    println!("LOGICAL_BYTES={}", record.logical_bytes);
    println!("NEW_STORED_BYTES={}", record.stored_bytes);
    println!("CHUNKS_WRITTEN={}", record.chunks_written);
    println!("CHUNKS_REUSED={}", record.chunks_reused);
    if record.logical_bytes > 0 {
        let ratio = (record.stored_bytes as f64 / record.logical_bytes as f64) * 100.0;
        println!("PHYSICAL_WRITE_RATIO_PERCENT={ratio:.2}");
    }
    Ok(())
}

fn command_restore(args: &[String]) -> Result<(), String> {
    let mut archive = None;
    let mut project = None;
    let mut explicit_to = None;
    let mut downloads = default_downloads_dir()?;
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--archive" => {
                i += 1;
                archive = Some(PathBuf::from(value(args, i, "--archive")?));
            }
            "--project" => {
                i += 1;
                project = Some(value(args, i, "--project")?.to_owned());
            }
            "--to" => {
                i += 1;
                explicit_to = Some(PathBuf::from(value(args, i, "--to")?));
            }
            "--downloads" => {
                i += 1;
                downloads = PathBuf::from(value(args, i, "--downloads")?);
            }
            flag => return Err(format!("unknown restore option: {flag}")),
        }
        i += 1;
    }
    let archive = archive.ok_or_else(|| "restore requires --archive PATH".to_owned())?;
    let layout = ForgeLayout::new(downloads);
    layout.ensure()?;
    let destination = explicit_to
        .or_else(|| project.as_deref().map(|name| layout.project_restored(name)))
        .unwrap_or_else(|| layout.root.join("Restored"));
    let restored =
        restore_cold_archive_with_store(&archive, &layout.coldpack_store(), &destination)?;
    println!("FORGECLEAN_RESTORE=PASS");
    println!("RESTORE_ROOT={}", destination.display());
    for path in restored {
        println!("RESTORED={}", path.display());
    }
    Ok(())
}

fn command_coldpack_status(args: &[String]) -> Result<(), String> {
    let downloads = parse_downloads_only(args, "coldpack-status")?;
    let layout = ForgeLayout::new(downloads);
    layout.ensure()?;
    let report = coldpack_status(&layout.root, &layout.coldpack_store(), unix_now_secs()?)?;
    println!("FORGECLEAN_COLDPACK_STATUS=PASS");
    print_coldpack_report(&report);
    Ok(())
}

fn command_coldpack_audit(args: &[String]) -> Result<(), String> {
    let downloads = parse_downloads_only(args, "coldpack-audit")?;
    let layout = ForgeLayout::new(downloads);
    layout.ensure()?;
    let report = coldpack_audit(&layout.root, &layout.coldpack_store(), unix_now_secs()?)?;
    println!("FORGECLEAN_COLDPACK_AUDIT=PASS");
    print_coldpack_report(&report);
    Ok(())
}

fn command_coldpack_gc(args: &[String]) -> Result<(), String> {
    let mut downloads = default_downloads_dir()?;
    let mut preview = false;
    let mut apply = false;
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--preview" => preview = true,
            "--apply" => apply = true,
            "--downloads" => {
                i += 1;
                downloads = PathBuf::from(value(args, i, "--downloads")?);
            }
            flag => return Err(format!("unknown coldpack-gc option: {flag}")),
        }
        i += 1;
    }
    if (preview && apply) || (!preview && !apply) {
        return Err("coldpack-gc requires exactly one of --preview or --apply".to_owned());
    }
    let layout = ForgeLayout::new(downloads);
    layout.ensure()?;
    let now = unix_now_secs()?;
    if preview {
        let report = coldpack_gc_preview(&layout.root, &layout.coldpack_store(), now)?;
        println!("FORGECLEAN_COLDPACK_GC_PREVIEW=PASS");
        print_coldpack_report(&report);
        println!("PURGE_EXECUTED=0");
    } else {
        let result = coldpack_gc_apply(&layout.root, &layout.coldpack_store(), now)?;
        println!("FORGECLEAN_COLDPACK_GC=PASS");
        print_coldpack_report(&result.report);
        println!("QUARANTINED_OBJECTS={}", result.quarantined_objects);
        println!("QUARANTINED_BYTES={}", result.quarantined_bytes);
        println!("PURGED_OBJECTS={}", result.purged_objects);
        println!("PURGED_BYTES={}", result.purged_bytes);
        println!("PURGE_EXECUTED=1");
    }
    Ok(())
}

fn print_coldpack_report(report: &ColdPackStoreReport) {
    println!("MANIFESTS={}", report.manifests);
    println!("LOGICAL_BYTES={}", report.logical_bytes);
    println!("UNIQUE_REFERENCES={}", report.unique_references);
    println!("ACTIVE_OBJECTS={}", report.active_objects);
    println!("ACTIVE_BYTES={}", report.active_bytes);
    println!("LIVE_OBJECTS={}", report.live_objects);
    println!("LIVE_BYTES={}", report.live_bytes);
    println!("ORPHAN_OBJECTS={}", report.orphan_objects);
    println!("ORPHAN_BYTES={}", report.orphan_bytes);
    println!("QUARANTINE_OBJECTS={}", report.quarantine_objects);
    println!("QUARANTINE_BYTES={}", report.quarantine_bytes);
    println!(
        "EXPIRED_QUARANTINE_OBJECTS={}",
        report.expired_quarantine_objects
    );
    println!(
        "EXPIRED_QUARANTINE_BYTES={}",
        report.expired_quarantine_bytes
    );
    if report.logical_bytes > 0 {
        let ratio = (report.active_bytes as f64 / report.logical_bytes as f64) * 100.0;
        println!("ACTIVE_PHYSICAL_RATIO_PERCENT={ratio:.2}");
    }
    for hash in &report.orphan_hashes {
        println!("ORPHAN_CANDIDATE={hash}");
    }
    for hash in &report.purge_hashes {
        println!("PURGE_CANDIDATE={hash}");
    }
}

fn unix_now_secs() -> Result<u64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|e| format!("system clock is before UNIX epoch: {e}"))
}

fn parse_organizer_options(
    args: &[String],
    allow_poll: bool,
) -> Result<(PathBuf, u64, u64, GuiSettings), String> {
    let settings = load_settings()?;
    let mut downloads = default_downloads_dir()?;
    let mut stable_seconds = settings.stable_seconds;
    let mut poll_seconds = settings.poll_seconds;
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--downloads" => {
                i += 1;
                downloads = PathBuf::from(value(args, i, "--downloads")?);
            }
            "--stable-seconds" => {
                i += 1;
                stable_seconds = value(args, i, "--stable-seconds")?
                    .parse()
                    .map_err(|e| format!("invalid --stable-seconds: {e}"))?;
            }
            "--poll-seconds" if allow_poll => {
                i += 1;
                poll_seconds = value(args, i, "--poll-seconds")?
                    .parse()
                    .map_err(|e| format!("invalid --poll-seconds: {e}"))?;
                if poll_seconds == 0 {
                    return Err("--poll-seconds must be at least 1".to_owned());
                }
            }
            flag => return Err(format!("unknown organizer option: {flag}")),
        }
        i += 1;
    }
    Ok((downloads, stable_seconds, poll_seconds, settings))
}

fn parse_downloads_only(args: &[String], command: &str) -> Result<PathBuf, String> {
    let mut downloads = default_downloads_dir()?;
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--downloads" => {
                i += 1;
                downloads = PathBuf::from(value(args, i, "--downloads")?);
            }
            flag => return Err(format!("unknown {command} option: {flag}")),
        }
        i += 1;
    }
    Ok(downloads)
}

fn default_downloads_dir() -> Result<PathBuf, String> {
    Ok(home_dir()?.join("Downloads"))
}

fn refuse_active_project_archive(layout: &ForgeLayout, source: &Path) -> Result<(), String> {
    let source = std::fs::canonicalize(source).map_err(|e| {
        format!(
            "cannot canonicalize archive source {}: {e}",
            source.display()
        )
    })?;
    let registry = ProjectRegistry::read(&layout.registry_path())?;
    for record in registry.records {
        if let Ok(active) = std::fs::canonicalize(&record.active_path)
            && source.starts_with(&active)
        {
            return Err(format!(
                "AETHER_GUARD=BLOCKED active project trees are never cold-compressed: {}",
                source.display()
            ));
        }
    }
    Ok(())
}

fn parse_auto_options(
    args: &[String],
    command: &str,
    allow_yes: bool,
) -> Result<(usize, u64, bool), String> {
    let mut keep_versions = 2usize;
    let mut partial_age_days = 7u64;
    let mut yes = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--keep" => {
                i += 1;
                keep_versions = value(args, i, "--keep")?
                    .parse()
                    .map_err(|e| format!("invalid --keep: {e}"))?;
            }
            "--partial-age-days" => {
                i += 1;
                partial_age_days = value(args, i, "--partial-age-days")?
                    .parse()
                    .map_err(|e| format!("invalid --partial-age-days: {e}"))?;
            }
            "--yes" if allow_yes => yes = true,
            flag => return Err(format!("unknown {command} option: {flag}")),
        }
        i += 1;
    }
    Ok((keep_versions, partial_age_days, yes))
}

fn required_manifest(args: &[String]) -> Result<PathBuf, String> {
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--manifest" {
            i += 1;
            return Ok(PathBuf::from(value(args, i, "--manifest")?));
        }
        i += 1;
    }
    Err("--manifest PATH is required".to_owned())
}

fn home_dir() -> Result<PathBuf, String> {
    if let Ok(sudo_user) = env::var("SUDO_USER")
        && !sudo_user.is_empty()
        && sudo_user != "root"
        && let Some(home) = passwd_home(&sudo_user)?
    {
        return Ok(home);
    }
    env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set; cannot place ForgeClean reports in ~/Downloads".to_owned())
}

fn passwd_home(user: &str) -> Result<Option<PathBuf>, String> {
    let passwd = std::fs::read_to_string("/etc/passwd")
        .map_err(|e| format!("cannot read /etc/passwd while resolving SUDO_USER: {e}"))?;
    for line in passwd.lines() {
        let mut fields = line.split(':');
        let Some(name) = fields.next() else {
            continue;
        };
        if name != user {
            continue;
        }
        let home = line
            .split(':')
            .nth(5)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| format!("SUDO_USER {user} has no home directory in /etc/passwd"))?;
        return Ok(Some(PathBuf::from(home)));
    }
    Ok(None)
}

fn ensure_downloads_parent(path: &Path) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "cannot resolve ~/Downloads parent".to_owned())?;
    if parent.is_dir() {
        Ok(())
    } else {
        Err(format!(
            "Downloads directory does not exist: {}",
            parent.display()
        ))
    }
}

fn write_offload_report(
    path: &Path,
    mount_point: &Path,
    report: &OffloadReport,
) -> Result<(), String> {
    let file = File::create(path)
        .map_err(|e| format!("cannot create offload report {}: {e}", path.display()))?;
    let mut out = BufWriter::new(file);
    writeln!(out, "FORGECLEAN-OFFLOAD-REPORT\t1").map_err(|e| e.to_string())?;
    writeln!(out, "MOUNT\t{}", mount_point.display()).map_err(|e| e.to_string())?;
    writeln!(out, "OFFLOADED_FILES\t{}", report.offloaded_files).map_err(|e| e.to_string())?;
    writeln!(out, "OFFLOADED_BYTES\t{}", report.offloaded_bytes).map_err(|e| e.to_string())?;
    for record in &report.records {
        writeln!(
            out,
            "ENTRY\t{}\t{}\t{}\t{}",
            record.bytes,
            record.sha256,
            record.source.display(),
            record.destination.display()
        )
        .map_err(|e| e.to_string())?;
    }
    for failure in &report.failures {
        writeln!(out, "FAILURE\t{failure}").map_err(|e| e.to_string())?;
    }
    out.flush().map_err(|e| e.to_string())
}

fn value<'a>(args: &'a [String], index: usize, flag: &str) -> Result<&'a str, String> {
    args.get(index)
        .map(String::as_str)
        .ok_or_else(|| format!("{flag} requires a value"))
}

fn print_help() {
    println!(
        "ForgeClean v{VERSION} - AetherForge persistent organizer, cold storage, cleanup + conditional offload"
    );
    println!();
    println!("PERSISTENT ORGANIZER:");
    println!("  forgeclean watch [--stable-seconds N] [--poll-seconds N]");
    println!("  forgeclean organize-once [--stable-seconds N]");
    println!("  canonical root: ~/Downloads/ForgeClean");
    println!();
    println!("PROJECT REGISTRY / BUILD REDIRECT:");
    println!("  forgeclean resolve-project PROJECT");
    println!("  forgeclean build PROJECT -- COMMAND [ARGS...]");
    println!("  forgeclean activate PROJECT --path PATH [--legacy PATH]");
    println!();
    println!("COLD STORAGE:");
    println!("  forgeclean archive --path PATH [--project PROJECT]");
    println!("  forgeclean restore --archive PATH [--project PROJECT] [--to PATH]");
    println!("  new archives: *.fcoldpack + shared deduplicated chunk store");
    println!("  legacy restore: *.fcold.tar.zst + SHA256 sidecar");
    println!();
    println!("COLDPACK STORE MAINTENANCE:");
    println!("  forgeclean coldpack-status [--downloads PATH]");
    println!("  forgeclean coldpack-audit [--downloads PATH]");
    println!("  forgeclean coldpack-gc --preview [--downloads PATH]");
    println!("  forgeclean coldpack-gc --apply [--downloads PATH]");
    println!("  GC is mark-and-sweep with a fixed 7-day quarantine");
    println!();
    println!("AUTOMATIC PACKAGE MODE (destructive; requires --yes):");
    println!("  forgeclean auto --yes [--keep N] [--partial-age-days N]");
    println!("  no external drive: AUTO_CLEAN (direct unlink, no Trash)");
    println!("  external drive:    AUTO_OFFLOAD (copy + SHA256 + fsync + source unlink)");
    println!();
    println!("STORAGE STATUS:");
    println!("  forgeclean storage-status");
    println!();
    println!("SYSTEM PACMAN PREVIEW (fixed /var/cache/pacman/pkg; never deletes):");
    println!("  forgeclean scan-system [--keep N] [--partial-age-days N]");
    println!("  manifest: ~/Downloads/ForgeClean-v1.0.1-PACMAN-BATCH.txt");
    println!();
    println!("CUSTOM SCAN (never deletes):");
    println!(
        "  forgeclean scan [--cache-dir PATH] [--keep N] [--partial-age-days N] [--manifest PATH]"
    );
    println!();
    println!("VERIFY / PURGE:");
    println!("  forgeclean verify --manifest PATH");
    println!("  forgeclean purge --manifest PATH --yes");
}

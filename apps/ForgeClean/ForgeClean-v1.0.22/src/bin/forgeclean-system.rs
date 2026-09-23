use forgeclean::adaptive::audit::AuditEngine;
use forgeclean::adaptive::baseline::BaselineStore;
use forgeclean::adaptive::benchmark::{
    BenchmarkEngine, BenchmarkKind, BenchmarkReport, BenchmarkRequest,
};
use forgeclean::adaptive::profile::{Profiler, ProfilerBudget};
use forgeclean::adaptive::regression::RegressionEngine;
use forgeclean::adaptive::store::AuditStore;
use forgeclean::adaptive::workload_guard::WorkloadGuard;
use forgeclean::orbital::{
    MigrationAction, OrbitalAction, run_migration_action, run_orbital_action,
};
use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

fn main() {
    if let Err(error) = run() {
        eprintln!("forgeclean-system: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    let home = home()?;
    let Some(command) = args.first().map(String::as_str) else {
        usage();
        return Ok(());
    };

    match command {
        "audit" => {
            let full = args.iter().any(|arg| arg == "--full");
            let engine = AuditEngine;
            let health = if full {
                engine.full()?
            } else {
                engine.quick()?
            };
            let store = AuditStore::for_home(&home);
            let current = store.write_current_health(&health)?;
            let history = store.write_history_health(&health)?;
            println!("FORGECLEAN_AUDIT=PASS");
            println!(
                "FORGECLEAN_AUDIT_MODE={}",
                if full { "FULL" } else { "QUICK" }
            );
            println!("FORGECLEAN_AUDIT_CURRENT={}", current.display());
            println!("FORGECLEAN_AUDIT_HISTORY={}", history.display());
            println!("{}", serde_json::to_string_pretty(&health)?);
        }
        "profile" => {
            let live = args.iter().any(|arg| arg == "--live");
            let interval_ms = option_u64(&args, "--interval-ms").unwrap_or(5_000);
            let duration_secs = option_u64(&args, "--duration-secs");
            let budget = ProfilerBudget::new(interval_ms, 5.0)
                .map_err(|e| format!("invalid profiler budget: {e}"))?;
            let profiler = Profiler::for_home(&home);
            if live {
                profiler.run_live(budget, duration_secs.map(Duration::from_secs))?;
            } else {
                let health = profiler.sample_once()?;
                println!("{}", serde_json::to_string_pretty(&health)?);
            }
        }
        "benchmark" => {
            let kind = args
                .get(1)
                .and_then(|v| BenchmarkKind::parse(v))
                .unwrap_or(BenchmarkKind::Quick);
            let request = if kind == BenchmarkKind::Quick {
                BenchmarkRequest::quick()
            } else {
                BenchmarkRequest::for_kind(kind)
            };
            let guard = WorkloadGuard::default();
            let report = BenchmarkEngine::new(&home).run(&request, &guard)?;
            let store = AuditStore::for_home(&home);
            let current = store.write_current("benchmark.json", &report)?;
            let history = store.write_history("benchmark.json", &report)?;
            println!("FORGECLEAN_BENCHMARK=PASS");
            println!("FORGECLEAN_BENCHMARK_CURRENT={}", current.display());
            println!("FORGECLEAN_BENCHMARK_HISTORY={}", history.display());
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        "baseline" => baseline_command(&home, &args)?,
        "regress" => regress_command(&home, &args)?,
        "report" => {
            let path = AuditStore::for_home(&home)
                .current_dir()
                .join("system-health.json");
            println!("{}", fs::read_to_string(path)?);
        }
        "orbital" => orbital_command(&home, &args)?,
        "migration" => migration_command(&home, &args)?,
        "pre-rebase" => pre_rebase_command(&home, &args)?,
        "project-storage" => project_storage_command(&home)?,
        "downloads-inventory" => downloads_inventory_command(&home, &args)?,
        "--version" | "version" => println!("forgeclean-system {}", env!("CARGO_PKG_VERSION")),
        _ => usage(),
    }
    Ok(())
}

fn baseline_command(home: &std::path::Path, args: &[String]) -> Result<(), Box<dyn Error>> {
    let Some(action) = args.get(1).map(String::as_str) else {
        return Err("baseline requires create or compare".into());
    };
    let Some(name) = args.get(2) else {
        return Err("baseline requires a name".into());
    };
    let store = AuditStore::for_home(home);
    let current: BenchmarkReport = store.read_current("benchmark.json")?;
    let baselines = BaselineStore::for_home(home);
    match action {
        "create" => {
            let path = baselines.create(name, &current)?;
            println!("FORGECLEAN_BASELINE_CREATE=PASS");
            println!("FORGECLEAN_BASELINE={}", path.display());
        }
        "compare" => {
            let baseline = baselines.load(name)?;
            let result = RegressionEngine::compare(&baseline, &current);
            println!("FORGECLEAN_BASELINE_COMPARE={:?}", result.class);
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        _ => return Err("baseline action must be create or compare".into()),
    }
    Ok(())
}

fn regress_command(home: &std::path::Path, args: &[String]) -> Result<(), Box<dyn Error>> {
    let Some(name) = args.get(1) else {
        return Err("regress requires a baseline name".into());
    };
    let store = AuditStore::for_home(home);
    let current: BenchmarkReport = store.read_current("benchmark.json")?;
    let baseline = BaselineStore::for_home(home).load(name)?;
    let result = RegressionEngine::compare(&baseline, &current);
    println!("FORGECLEAN_REGRESSION={:?}", result.class);
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

fn orbital_command(home: &std::path::Path, args: &[String]) -> Result<(), Box<dyn Error>> {
    let action = args
        .get(1)
        .and_then(|value| OrbitalAction::parse(value))
        .ok_or("orbital requires status|sync|full|retry|pause|resume|diagnostic")?;
    print!("{}", run_orbital_action(home, action)?);
    Ok(())
}

fn migration_command(home: &std::path::Path, args: &[String]) -> Result<(), Box<dyn Error>> {
    let action = args
        .get(1)
        .and_then(|value| MigrationAction::parse(value))
        .ok_or("migration requires status|scan|upload|verify|restore|sync|inventory")?;
    print!("{}", run_migration_action(home, action)?);
    Ok(())
}

fn downloads_inventory_command(
    home: &std::path::Path,
    args: &[String],
) -> Result<(), Box<dyn Error>> {
    let downloads = home.join("Downloads");
    let details = args.iter().any(|arg| arg == "--details");
    let summary_only = args.iter().any(|arg| arg == "--summary-only");
    let projects_only = args.iter().any(|arg| arg == "--projects-only");
    let report = if details {
        forgeclean::downloads_inventory::scan_downloads_inventory(&downloads)?
    } else {
        forgeclean::downloads_inventory::scan_downloads_inventory_summary(&downloads)?
    };
    println!("FORGECLEAN_DOWNLOADS_INVENTORY=PASS");
    println!("ROOT={}", report.root.display());
    println!("SCAN_THREADS={}", report.scan_threads);
    println!("ACCOUNTING_MODE=PATH_BASED_FAST");
    println!("TOTAL_ENTRIES={}", report.total.entry_paths);
    println!("TOTAL_FILES={}", report.total.file_paths);
    println!("TOTAL_DIRECTORIES={}", report.total.directory_paths);
    println!("TOTAL_SYMLINKS={}", report.total.symlink_paths);
    println!("TOTAL_OTHER={}", report.total.other_paths);
    println!("TOTAL_ALLOCATED_BYTES={}", report.total.allocated_bytes);
    println!("TOTAL_LOGICAL_BYTES={}", report.total.logical_bytes);
    println!("GENERAL_ENTRIES={}", report.general.entry_paths);
    println!("GENERAL_FILES={}", report.general.file_paths);
    println!("GENERAL_ALLOCATED_BYTES={}", report.general.allocated_bytes);
    println!("PROJECT_ENTRIES={}", report.project_files.entry_paths);
    println!("PROJECT_FILES={}", report.project_files.file_paths);
    println!(
        "PROJECT_ALLOCATED_BYTES={}",
        report.project_files.allocated_bytes
    );
    println!("PROJECT_COUNT={}", report.projects.len());
    println!("SCAN_ERRORS={}", report.scan_errors);
    println!("DETAILS={}", if details { "ON" } else { "OFF" });

    let project_limit = if projects_only {
        usize::MAX
    } else if summary_only {
        0
    } else {
        20
    };
    for project in report.projects.iter().take(project_limit) {
        println!(
            "PROJECT={}\tENTRIES={}\tFILES={}\tDIRECTORIES={}\tSYMLINKS={}\tALLOCATED_BYTES={}\tLOGICAL_BYTES={}",
            escape_inventory_field(&project.name),
            project.summary.entry_paths,
            project.summary.file_paths,
            project.summary.directory_paths,
            project.summary.symlink_paths,
            project.summary.allocated_bytes,
            project.summary.logical_bytes
        );
    }
    println!(
        "PROJECTS_SHOWN={}",
        report.projects.len().min(project_limit)
    );

    if details {
        for entry in &report.entries {
            if projects_only && entry.project.is_none() {
                continue;
            }
            let lane = if entry.project.is_some() {
                "PROJECT"
            } else {
                "GENERAL"
            };
            let project = entry.project.as_deref().unwrap_or("");
            let class = entry.project_class.as_deref().unwrap_or("");
            let target = entry
                .symlink_target
                .as_ref()
                .map(|path| path.to_string_lossy().into_owned())
                .unwrap_or_default();
            println!(
                "ENTRY={}\tTYPE={}\tPROJECT={}\tCLASS={}\tALLOCATED_BYTES={}\tLOGICAL_BYTES={}\tMTIME_UNIX={}\tUID={}\tGID={}\tMODE={:04o}\tEXT={}\tPATH={}\tSYMLINK_TARGET={}",
                lane,
                entry.kind,
                escape_inventory_field(project),
                escape_inventory_field(class),
                entry.allocated_bytes,
                entry.logical_bytes,
                entry.modified_unix,
                entry.uid,
                entry.gid,
                entry.mode,
                escape_inventory_field(&entry.extension),
                escape_inventory_field(&entry.relative_path.to_string_lossy()),
                escape_inventory_field(&target)
            );
        }
    }
    Ok(())
}

fn escape_inventory_field(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}
fn project_storage_command(home: &std::path::Path) -> Result<(), Box<dyn Error>> {
    let downloads = home.join("Downloads");
    let report = forgeclean::project_storage::scan_project_download_storage(&downloads)?;
    println!("FORGECLEAN_PROJECT_STORAGE=PASS");
    println!("ALLOCATED_BYTES={}", report.allocated_bytes);
    println!("LOGICAL_BYTES={}", report.logical_bytes);
    println!("ARTIFACT_PATHS={}", report.artifact_paths);
    println!("UNIQUE_FILES={}", report.unique_files);
    println!("DEDUPLICATED_PATHS={}", report.deduplicated_paths);
    println!("UNROUTED_ARTIFACT_PATHS={}", report.unrouted_artifact_paths);
    println!("EXCLUDED_GENERATED_DIRS={}", report.excluded_generated_dirs);
    println!("SYMLINKS_SKIPPED={}", report.symlinks_skipped);
    println!("SCAN_ERRORS={}", report.scan_errors);
    for project in &report.projects {
        println!(
            "PROJECT={}\tALLOCATED_BYTES={}\tLOGICAL_BYTES={}\tARTIFACT_PATHS={}\tUNIQUE_FILES={}",
            project.name,
            project.allocated_bytes,
            project.logical_bytes,
            project.artifact_paths,
            project.unique_files
        );
    }
    Ok(())
}
fn pre_rebase_command(home: &std::path::Path, args: &[String]) -> Result<(), Box<dyn Error>> {
    use forgeclean::pre_rebase::{
        PreRebasePolicy, STORAGE_CRITICAL_PERCENT, STORAGE_PRESSURE_PERCENT, auto_reason,
        current_boot_id, now_unix_secs, read_state, scan, state_path,
    };

    let action = args.get(1).map(String::as_str).unwrap_or("status");
    let storage_used = filesystem_used_percent(home).unwrap_or(0);
    match action {
        "status" => {
            let policy = PreRebasePolicy::for_home(home)?;
            let state = read_state(home)?;
            println!("FORGECLEAN_PRE_REBASE_STATUS=PASS");
            println!("CUTOFF_UNIX_SECS={}", policy.cutoff_unix_secs);
            println!("ROOTS={}", policy.roots.len());
            println!("STORAGE_USED_PERCENT={storage_used}");
            println!("STORAGE_PRESSURE_PERCENT={STORAGE_PRESSURE_PERCENT}");
            println!("STORAGE_CRITICAL_PERCENT={STORAGE_CRITICAL_PERCENT}");
            println!(
                "LAST_RUN_UNIX_SECS={}",
                state
                    .last_run_unix_secs
                    .map_or_else(|| "NEVER".to_owned(), |value| value.to_string())
            );
            println!(
                "CUMULATIVE_DELETED_FILES={}",
                state.cumulative_deleted_files
            );
            println!(
                "CUMULATIVE_DELETED_BYTES={}",
                state.cumulative_deleted_bytes
            );
            println!("STATE_FILE={}", state_path(home).display());
        }
        "scan" => {
            let policy = PreRebasePolicy::for_home(home)?;
            let report = scan(&policy);
            print_pre_rebase_scan(&report, storage_used);
        }
        "apply" => {
            require_yes(args, "pre-rebase apply")?;
            let reason = option_string(args, "--reason").unwrap_or_else(|| "MANUAL".to_owned());
            run_pre_rebase_sweep(home, storage_used, &reason)?;
        }
        "auto" => {
            require_yes(args, "pre-rebase auto")?;
            let state = read_state(home)?;
            let boot_id = current_boot_id();
            let now = now_unix_secs();
            if let Some(reason) = auto_reason(&state, now, boot_id.as_deref(), storage_used) {
                let reason = reason.as_label();
                println!("FORGECLEAN_PRE_REBASE_AUTO=RUN");
                println!("AUTO_REASON={reason}");
                run_pre_rebase_sweep(home, storage_used, &reason)?;
            } else {
                println!("FORGECLEAN_PRE_REBASE_AUTO=SKIP");
                println!("STORAGE_USED_PERCENT={storage_used}");
            }
        }
        _ => return Err("pre-rebase requires status|scan|apply|auto".into()),
    }
    Ok(())
}

fn run_pre_rebase_sweep(
    home: &std::path::Path,
    storage_used: u8,
    reason: &str,
) -> Result<(), Box<dyn Error>> {
    use forgeclean::pre_rebase::{
        PreRebasePolicy, STORAGE_CRITICAL_PERCENT, current_boot_id, now_unix_secs, purge,
        read_state, scan, write_state,
    };

    let policy = PreRebasePolicy::for_home(home)?;
    let report = scan(&policy);
    print_pre_rebase_scan(&report, storage_used);
    let purge_report = purge(&policy, &report.candidates);

    let mut state = read_state(home)?;
    state.last_run_unix_secs = Some(now_unix_secs());
    state.last_boot_id = current_boot_id();
    state.last_reason = Some(reason.to_owned());
    state.last_deleted_files = purge_report.deleted_files;
    state.last_deleted_bytes = purge_report.deleted_bytes;
    state.cumulative_deleted_files = state
        .cumulative_deleted_files
        .saturating_add(purge_report.deleted_files);
    state.cumulative_deleted_bytes = state
        .cumulative_deleted_bytes
        .saturating_add(purge_report.deleted_bytes);
    let state_file = write_state(home, &state)?;

    println!("FORGECLEAN_PRE_REBASE_DELETE=PASS");
    println!("DELETE_REASON={reason}");
    println!("DELETED_FILES={}", purge_report.deleted_files);
    println!("DELETED_BYTES={}", purge_report.deleted_bytes);
    println!("SKIPPED_CHANGED={}", purge_report.changed_since_scan);
    println!("SKIPPED_PROTECTED={}", purge_report.protected_since_scan);
    println!("SKIPPED_OUTSIDE_ROOTS={}", purge_report.outside_roots);
    println!("DELETE_ERRORS={}", purge_report.delete_errors);
    println!("STATE_FILE={}", state_file.display());
    if storage_used >= STORAGE_CRITICAL_PERCENT {
        println!("FORGECLEAN_STORAGE_CRITICAL=1");
    }
    Ok(())
}

fn print_pre_rebase_scan(report: &forgeclean::pre_rebase::PreRebaseScanReport, storage_used: u8) {
    println!("FORGECLEAN_PRE_REBASE_SCAN=PASS");
    println!("ROOTS={}", report.roots.len());
    println!("CANDIDATE_FILES={}", report.candidates.len());
    println!("RECOVERABLE_BYTES={}", report.candidate_bytes);
    println!("PROTECTED_ENTRIES={}", report.protected_entries);
    println!("SYMLINKS_SKIPPED={}", report.symlinks_skipped);
    println!("NON_OWNED_SKIPPED={}", report.non_owned_skipped);
    println!("SCAN_ERRORS={}", report.scan_errors);
    println!("STORAGE_USED_PERCENT={storage_used}");
}

fn require_yes(args: &[String], command: &str) -> Result<(), Box<dyn Error>> {
    if args.iter().any(|arg| arg == "--yes") {
        Ok(())
    } else {
        Err(format!("{command} requires explicit --yes").into())
    }
}

fn option_string(args: &[String], name: &str) -> Option<String> {
    let index = args.iter().position(|arg| arg == name)?;
    args.get(index + 1).cloned()
}

fn filesystem_used_percent(path: &std::path::Path) -> Result<u8, Box<dyn Error>> {
    let output = std::process::Command::new("df")
        .arg("-P")
        .arg(path)
        .output()?;
    if !output.status.success() {
        return Err("df failed while reading storage pressure".into());
    }
    let text = String::from_utf8(output.stdout)?;
    let line = text.lines().last().ok_or("df returned no filesystem row")?;
    let percent = line
        .split_whitespace()
        .nth(4)
        .ok_or("df filesystem row has no percentage")?
        .trim_end_matches('%')
        .parse::<u8>()?;
    Ok(percent)
}
fn option_u64(args: &[String], name: &str) -> Option<u64> {
    let index = args.iter().position(|arg| arg == name)?;
    args.get(index + 1)?.parse().ok()
}

fn home() -> Result<PathBuf, Box<dyn Error>> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".into())
}

fn usage() {
    eprintln!(
        "usage:
  forgeclean-system audit [--full]
  forgeclean-system profile [--live] [--interval-ms N] [--duration-secs N]
  forgeclean-system benchmark [quick|full|cpu|memory|storage|network|audio|beacn|mixed]
  forgeclean-system baseline create NAME
  forgeclean-system baseline compare NAME
  forgeclean-system regress NAME
  forgeclean-system report
  forgeclean-system orbital [status|sync|full|retry|pause|resume|diagnostic]\n  forgeclean-system migration [status|scan|upload|verify|restore|sync|inventory]\n  forgeclean-system --version"
    );
}

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
        "--version" | "version" => println!("forgeclean-system 1.0.4"),
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

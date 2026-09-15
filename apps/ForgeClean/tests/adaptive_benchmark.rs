use forgeclean::adaptive::benchmark::{BenchmarkEngine, BenchmarkError, BenchmarkRequest};
use forgeclean::adaptive::workload_guard::{ProtectedReason, WorkloadGuard};

#[test]
fn benchmark_refuses_protected_workload() {
    let home = tempfile_home();
    let guard = WorkloadGuard::with_forced_reason(ProtectedReason::Streaming);
    let result = BenchmarkEngine::new(&home).run(&BenchmarkRequest::quick(), &guard);
    assert!(matches!(result, Err(BenchmarkError::ProtectedWorkload(_))));
}

fn tempfile_home() -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!("forgeclean-adaptive-bench-{}", std::process::id()));
    std::fs::create_dir_all(&p).unwrap();
    p
}

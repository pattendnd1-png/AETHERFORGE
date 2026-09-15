use forgeclean::adaptive::profile::ProfilerBudget;

#[test]
fn profiler_rejects_subsecond_polling() {
    assert!(ProfilerBudget::new(250, 5.0).is_err());
}

#[test]
fn profiler_accepts_low_cost_interval() {
    let budget = ProfilerBudget::new(5_000, 5.0).unwrap();
    assert_eq!(budget.interval_ms(), 5_000);
}

use forgeclean::adaptive::benchmark::{BenchmarkKind, BenchmarkMetric, BenchmarkReport};
use forgeclean::adaptive::health::{HealthIncident, SystemHealth};
use forgeclean::adaptive::regression::{RegressionClass, RegressionEngine};

fn report(xruns: f64) -> BenchmarkReport {
    BenchmarkReport {
        schema_version: 1,
        started_at_unix_ms: 1,
        ended_at_unix_ms: 2,
        kind: BenchmarkKind::Audio,
        metrics: vec![BenchmarkMetric {
            name: "pipewire_xruns".into(),
            value: xruns,
            unit: "count".into(),
        }],
        health: SystemHealth {
            schema_version: 1,
            sampled_at_unix_ms: 2,
            samples: vec![],
            incidents: Vec::<HealthIncident>::new(),
        },
    }
}

#[test]
fn audio_xrun_increase_is_critical_regression() {
    let r = RegressionEngine::compare(&report(0.0), &report(1.0));
    assert_eq!(r.class, RegressionClass::Critical);
}

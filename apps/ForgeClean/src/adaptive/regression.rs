use crate::adaptive::benchmark::{BenchmarkMetric, BenchmarkReport};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum RegressionClass {
    Pass,
    Warn,
    Regression,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RegressionReport {
    pub class: RegressionClass,
    pub reasons: Vec<String>,
}

pub struct RegressionEngine;

impl RegressionEngine {
    pub fn compare(baseline: &BenchmarkReport, candidate: &BenchmarkReport) -> RegressionReport {
        let mut class = RegressionClass::Pass;
        let mut reasons = Vec::new();

        for incident in &candidate.health.incidents {
            if incident.fatal_for_tuning {
                class = RegressionClass::Critical;
                reasons.push(format!("fatal health incident: {}", incident.code));
            }
        }

        if metric(candidate, "pipewire_xruns") > metric(baseline, "pipewire_xruns") {
            class = RegressionClass::Critical;
            reasons.push("PipeWire xruns increased".into());
        }
        if metric(candidate, "beacn_dropouts") > metric(baseline, "beacn_dropouts") {
            class = RegressionClass::Critical;
            reasons.push("BEACN dropouts increased".into());
        }

        for name in [
            "cpu_integer_ops_s",
            "memory_copy_mib_s",
            "storage_read_mib_s",
            "storage_write_mib_s",
        ] {
            let before = metric(baseline, name);
            let after = metric(candidate, name);
            if before <= 0.0 || after <= 0.0 {
                continue;
            }
            let ratio = after / before;
            if ratio < 0.90 && class < RegressionClass::Regression {
                class = RegressionClass::Regression;
                reasons.push(format!("{name} regressed by more than 10%"));
            } else if ratio < 0.97 && class < RegressionClass::Warn {
                class = RegressionClass::Warn;
                reasons.push(format!("{name} regressed by more than 3%"));
            }
        }

        RegressionReport { class, reasons }
    }
}

fn metric(report: &BenchmarkReport, name: &str) -> f64 {
    report
        .metrics
        .iter()
        .find_map(|BenchmarkMetric { name: n, value, .. }| (n == name).then_some(*value))
        .unwrap_or(0.0)
}

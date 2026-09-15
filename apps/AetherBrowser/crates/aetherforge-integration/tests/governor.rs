use aetherforge_integration::{AetherForgeGovernorBridge, GovernorSignal, WorkloadClass};

struct FixedBridge;
impl AetherForgeGovernorBridge for FixedBridge {
    fn current_signal(&self) -> GovernorSignal {
        GovernorSignal {
            workload: WorkloadClass::Gaming,
            pressure_percent: 72,
        }
    }
}

#[test]
fn bridge_exposes_provider_neutral_governor_signal() {
    let signal = FixedBridge.current_signal();
    assert_eq!(signal.workload, WorkloadClass::Gaming);
    assert_eq!(signal.pressure_percent, 72);
}

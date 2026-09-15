#![forbid(unsafe_code)]
//! Replaceable AetherForge platform integration boundary.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkloadClass {
    Desktop,
    Gaming,
    Streaming,
    Creation,
    Compilation,
    Mixed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GovernorSignal {
    pub workload: WorkloadClass,
    pub pressure_percent: u8,
}

pub trait AetherForgeGovernorBridge {
    fn current_signal(&self) -> GovernorSignal;
}

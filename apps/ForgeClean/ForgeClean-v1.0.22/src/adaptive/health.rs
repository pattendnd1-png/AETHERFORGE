use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SystemHealth {
    pub schema_version: u32,
    pub sampled_at_unix_ms: u64,
    pub samples: Vec<HealthSample>,
    pub incidents: Vec<HealthIncident>,
}

impl SystemHealth {
    pub fn empty_now() -> Self {
        Self {
            schema_version: 1,
            sampled_at_unix_ms: unix_ms(),
            samples: Vec::new(),
            incidents: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", content = "value")]
pub enum HealthSample {
    CpuLoad1(f64),
    CpuTemperatureC(f64),
    IoPsiAvg10(f64),
    MemoryAvailableBytes(u64),
    GpuBusyPct(f64),
    GpuJunctionC(f64),
    NetworkRxBytes(u64),
    NetworkTxBytes(u64),
    NetworkRetransmits(u64),
    PipeWireXruns(u64),
    BeacnConnected(bool),
    BeacnDropouts(u64),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HealthIncident {
    pub code: String,
    pub detail: String,
    pub fatal_for_tuning: bool,
}

pub fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

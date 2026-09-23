//! Performance mode detection for adaptive UI

use std::fs;

pub struct PerformanceMode {
    pub ram_total_gb: u64,
    pub ram_free_gb: u64,
    pub mode: String,
}

impl PerformanceMode {
    pub fn detect() -> Self {
        let meminfo = fs::read_to_string("/proc/meminfo").unwrap_or_default();
        let ram_total_mb = meminfo
            .lines()
            .find(|l| l.starts_with("MemTotal:"))
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(8000);
        
        let ram_free_mb = meminfo
            .lines()
            .find(|l| l.starts_with("MemAvailable:"))
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(ram_total_mb / 2);
        
        let ram_total_gb = (ram_total_mb / 1024) as u64;
        let ram_free_gb = (ram_free_mb / 1024) as u64;
        
        let mode = if ram_total_gb < 4 {
            "minimal".to_string()
        } else if ram_total_gb < 8 {
            "light".to_string()
        } else {
            "full".to_string()
        };
        
        Self {
            ram_total_gb,
            ram_free_gb,
            mode,
        }
    }
    
    pub fn should_disable_effects(&self) -> bool {
        self.ram_free_gb < 2
    }
}

use crate::adaptive::audit::AuditEngine;
use crate::adaptive::health::SystemHealth;
use crate::adaptive::store::AuditStore;
use std::io;
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy)]
pub struct ProfilerBudget {
    interval_ms: u64,
    max_cpu_pct: f64,
}

impl ProfilerBudget {
    pub fn new(interval_ms: u64, max_cpu_pct: f64) -> Result<Self, String> {
        if interval_ms < 1_000 {
            return Err("profiler interval must be at least 1000 ms".into());
        }
        if !(0.0..=5.0).contains(&max_cpu_pct) || max_cpu_pct == 0.0 {
            return Err("profiler CPU budget must be >0 and <=5 percent".into());
        }
        Ok(Self {
            interval_ms,
            max_cpu_pct,
        })
    }

    pub fn interval_ms(self) -> u64 {
        self.interval_ms
    }

    pub fn max_cpu_pct(self) -> f64 {
        self.max_cpu_pct
    }
}

#[derive(Debug, Clone)]
pub struct Profiler {
    engine: AuditEngine,
    store: AuditStore,
}

impl Profiler {
    pub fn for_home(home: &Path) -> Self {
        Self {
            engine: AuditEngine,
            store: AuditStore::for_home(home),
        }
    }

    pub fn sample_once(&self) -> io::Result<SystemHealth> {
        let health = self.engine.quick()?;
        self.store.write_current_health(&health)?;
        Ok(health)
    }

    pub fn run_live(&self, budget: ProfilerBudget, duration: Option<Duration>) -> io::Result<()> {
        let started = Instant::now();
        loop {
            self.sample_once()?;
            if duration.is_some_and(|limit| started.elapsed() >= limit) {
                return Ok(());
            }
            thread::sleep(Duration::from_millis(budget.interval_ms()));
        }
    }
}

use reforge_core::{DiagnosticEvent, DiagnosticLevel};
use std::collections::VecDeque;

pub const MAX_DIAGNOSTIC_EVENTS: usize = 200;

#[derive(Debug, Default)]
pub struct DiagnosticRing {
    events: VecDeque<DiagnosticEvent>,
}

impl DiagnosticRing {
    pub fn push(&mut self, event: DiagnosticEvent) {
        if self.events.len() >= MAX_DIAGNOSTIC_EVENTS {
            self.events.pop_front();
        }
        self.events.push_back(event);
    }

    pub fn record(
        &mut self,
        unix_ms: u64,
        level: DiagnosticLevel,
        component: impl Into<String>,
        message: impl Into<String>,
        device_key: Option<String>,
    ) {
        self.push(DiagnosticEvent {
            unix_ms,
            level,
            component: component.into(),
            message: message.into(),
            device_key,
        });
    }

    pub fn snapshot(&self) -> Vec<DiagnosticEvent> {
        self.events.iter().cloned().collect()
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_is_bounded_and_clearable() {
        let mut ring = DiagnosticRing::default();
        for index in 0..=MAX_DIAGNOSTIC_EVENTS {
            ring.record(index as u64, DiagnosticLevel::Info, "test", index.to_string(), None);
        }
        let items = ring.snapshot();
        assert_eq!(items.len(), MAX_DIAGNOSTIC_EVENTS);
        assert_eq!(items.first().unwrap().message, "1");
        ring.clear();
        assert!(ring.snapshot().is_empty());
    }
}

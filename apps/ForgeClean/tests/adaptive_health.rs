use forgeclean::adaptive::health::{HealthSample, SystemHealth};

#[test]
fn system_health_serializes_stably() {
    let health = SystemHealth {
        schema_version: 1,
        sampled_at_unix_ms: 42,
        samples: vec![HealthSample::CpuLoad1(1.25)],
        incidents: vec![],
    };
    let json = serde_json::to_string(&health).unwrap();
    assert!(json.contains(r#""schema_version":1"#));
    assert!(json.contains("CpuLoad1"));
}

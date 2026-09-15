use aether_gaming::{GameProvider, GameSummary, GamingHub, GamingTelemetry};

struct Provider;
impl GameProvider for Provider {
    fn provider_id(&self) -> &str {
        "local"
    }
    fn games(&self) -> Vec<GameSummary> {
        vec![GameSummary {
            id: "game-1".into(),
            title: "Aether Test Game".into(),
            installed: true,
        }]
    }
}

#[test]
fn gaming_hub_aggregates_installed_games_and_browser_telemetry() {
    let provider = Provider;
    let mut hub = GamingHub::from_providers(&[&provider]);
    hub.set_telemetry(GamingTelemetry {
        system_cpu_percent: 42,
        system_gpu_percent: 70,
        system_memory_mib: 8192,
        browser_cpu_percent: 8,
        browser_gpu_percent: 4,
        browser_memory_mib: 1024,
    });
    assert_eq!(hub.games().len(), 1);
    assert_eq!(hub.installed_games().count(), 1);
    assert_eq!(hub.telemetry().browser_memory_mib, 1024);
}

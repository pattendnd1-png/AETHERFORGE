#![forbid(unsafe_code)]
//! Gaming-hub and local-game provider boundaries.

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GameSummary {
    pub id: String,
    pub title: String,
    pub installed: bool,
}

pub trait GameProvider {
    fn provider_id(&self) -> &str;
    fn games(&self) -> Vec<GameSummary>;
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GamingTelemetry {
    pub system_cpu_percent: u8,
    pub system_gpu_percent: u8,
    pub system_memory_mib: u32,
    pub browser_cpu_percent: u8,
    pub browser_gpu_percent: u8,
    pub browser_memory_mib: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HubGame {
    pub provider_id: String,
    pub game: GameSummary,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GamingHub {
    games: Vec<HubGame>,
    telemetry: GamingTelemetry,
    active_game_id: Option<String>,
}

impl GamingHub {
    #[must_use]
    pub fn from_providers(providers: &[&dyn GameProvider]) -> Self {
        let games = providers
            .iter()
            .flat_map(|provider| {
                let provider_id = provider.provider_id().to_owned();
                provider.games().into_iter().map(move |game| HubGame {
                    provider_id: provider_id.clone(),
                    game,
                })
            })
            .collect();
        Self {
            games,
            telemetry: GamingTelemetry::default(),
            active_game_id: None,
        }
    }

    pub fn set_telemetry(&mut self, telemetry: GamingTelemetry) {
        self.telemetry = telemetry;
    }

    pub fn set_active_game(&mut self, game_id: Option<String>) {
        self.active_game_id = game_id;
    }

    #[must_use]
    pub fn games(&self) -> &[HubGame] {
        &self.games
    }

    pub fn installed_games(&self) -> impl Iterator<Item = &HubGame> {
        self.games.iter().filter(|entry| entry.game.installed)
    }

    #[must_use]
    pub const fn telemetry(&self) -> &GamingTelemetry {
        &self.telemetry
    }

    #[must_use]
    pub fn active_game_id(&self) -> Option<&str> {
        self.active_game_id.as_deref()
    }
}

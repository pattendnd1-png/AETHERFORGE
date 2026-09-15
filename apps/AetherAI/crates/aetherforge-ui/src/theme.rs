use crate::{AetherForgeVisualContract, Rgba};

#[derive(Debug, Clone, PartialEq)]
pub struct DragonGlassTheme {
    contract: AetherForgeVisualContract,
    pub surface: Rgba,
    pub value_text: Rgba,
    pub label_text: Rgba,
    pub heading_text: Rgba,
}

impl DragonGlassTheme {
    pub fn terminal_canonical() -> Self {
        let contract = AetherForgeVisualContract::terminal_canonical();
        Self {
            surface: Rgba::new(0x0A, 0x06, 0x16, contract.main_surface_alpha),
            value_text: contract.value_text,
            label_text: contract.label_text,
            heading_text: contract.heading_text,
            contract,
        }
    }

    pub fn contract(&self) -> &AetherForgeVisualContract {
        &self.contract
    }
}

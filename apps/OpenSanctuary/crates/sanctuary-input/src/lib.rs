#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputAction {
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    Primary,
    Secondary,
    Menu,
    Quit,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InputSnapshot {
    pressed: Vec<InputAction>,
}

impl InputSnapshot {
    pub fn press(&mut self, action: InputAction) {
        if !self.pressed.contains(&action) {
            self.pressed.push(action);
        }
    }

    pub fn is_pressed(&self, action: InputAction) -> bool {
        self.pressed.contains(&action)
    }

    pub fn clear(&mut self) {
        self.pressed.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_tracks_native_actions_without_platform_keycodes() {
        let mut input = InputSnapshot::default();
        input.press(InputAction::Quit);
        assert!(input.is_pressed(InputAction::Quit));
        input.clear();
        assert!(!input.is_pressed(InputAction::Quit));
    }
}

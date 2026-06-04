use winit::keyboard::KeyCode;

/// High-level actions the application understands.
/// `app.rs` maps raw winit events to these; nothing below this layer knows about KeyCode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppAction {
    Quit,
    ToggleFullscreen,
    ExitFullscreen,
}

impl AppAction {
    pub fn from_key(key: KeyCode) -> Option<Self> {
        match key {
            KeyCode::KeyQ => Some(Self::Quit),
            KeyCode::F11 => Some(Self::ToggleFullscreen),
            KeyCode::Escape => Some(Self::ExitFullscreen),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn q_maps_to_quit() {
        assert_eq!(AppAction::from_key(KeyCode::KeyQ), Some(AppAction::Quit));
    }

    #[test]
    fn f11_maps_to_fullscreen() {
        assert_eq!(
            AppAction::from_key(KeyCode::F11),
            Some(AppAction::ToggleFullscreen)
        );
    }

    #[test]
    fn escape_maps_to_exit_fullscreen() {
        assert_eq!(
            AppAction::from_key(KeyCode::Escape),
            Some(AppAction::ExitFullscreen)
        );
    }

    #[test]
    fn unbound_key_returns_none() {
        assert_eq!(AppAction::from_key(KeyCode::KeyZ), None);
    }
}

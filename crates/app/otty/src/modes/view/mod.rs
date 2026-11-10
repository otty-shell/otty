pub(crate) mod mode_menu;
pub(crate) mod mode_switcher;

use otty_app_domain::mode::Mode;
use otty_app_i18n::Key;

/// Translation key of the mode's name in the switcher and the menu.
fn name_key(mode: Mode) -> Key {
    match mode {
        Mode::Terminal => Key::ModeTerminal,
        Mode::Containers => Key::ModeContainers,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_mode_has_its_own_name() {
        assert_eq!(name_key(Mode::Terminal), Key::ModeTerminal);
        assert_eq!(name_key(Mode::Containers), Key::ModeContainers);
    }
}

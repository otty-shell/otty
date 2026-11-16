//! The key bindings every terminal registers: one per app shortcut.
//!
//! They come from the same table the app keyboard subscription matches,
//! `AppShortcut::keys_on` in `otty-app-domain`. A focused terminal matches
//! a key press against them, reports the shortcut as
//! [`otty_ui_term::Event::BindingActionDispatched`] and keeps the key, so
//! the keyboard subscription never sees it and the shortcut runs once.
//! They replace the terminal's own binding for the same keys (for example
//! Ctrl Shift T, which the terminal would otherwise send as `^T`). The
//! block keys hold only on the normal screen: while a full-screen program
//! is on (the alternate screen), the terminal sends them to the program as
//! before.

use iced::keyboard::Modifiers;
use iced::keyboard::key::{Code, Named};
use otty_app_domain::keys::{
    KeyModifiers, Platform, ShortcutKey, ShortcutKeys,
};
use otty_app_domain::shortcuts::AppShortcut;
use otty_ui_term::SurfaceMode;
use otty_ui_term::bindings::{BindingAction, InputKind, KeyboardBinding};

/// A key binding of a terminal with the app shortcut it reports.
pub(crate) type ShortcutBinding = (KeyboardBinding, BindingAction<AppShortcut>);

/// Return the bindings every terminal registers: one per app shortcut, for
/// the current platform.
pub(crate) fn terminal_bindings() -> Vec<ShortcutBinding> {
    bindings_on(Platform::current())
}

/// Return the bindings of every app shortcut on `platform`.
fn bindings_on(platform: Platform) -> Vec<ShortcutBinding> {
    AppShortcut::ALL
        .into_iter()
        .map(|shortcut| {
            let binding = keyboard_binding(
                shortcut.keys_on(platform),
                screens_left_to_the_program(shortcut),
            );

            (binding, BindingAction::Action(shortcut))
        })
        .collect()
}

/// The screens on which the keys of `shortcut` reach the program instead
/// of the app. A full-screen program (the alternate screen: `vim`, `tmux`)
/// has no blocks, so the block keys are its own there; every other shortcut
/// belongs to the app on every screen.
fn screens_left_to_the_program(shortcut: AppShortcut) -> SurfaceMode {
    match shortcut {
        AppShortcut::PreviousBlock | AppShortcut::NextBlock => {
            SurfaceMode::ALT_SCREEN
        },
        _ => SurfaceMode::empty(),
    }
}

/// The binding of `keys`, which holds on every screen but `excluded`: a
/// digit-row key by its place on the keyboard, as the keyboard subscription
/// matches it, an arrow by its name, any other key by its character.
fn keyboard_binding(
    keys: ShortcutKeys,
    excluded: SurfaceMode,
) -> KeyboardBinding {
    let target = match keys.key() {
        ShortcutKey::Character(character) => {
            InputKind::Char(character.to_string())
        },
        ShortcutKey::Digit1 => InputKind::Physical(Code::Digit1),
        ShortcutKey::Digit2 => InputKind::Physical(Code::Digit2),
        ShortcutKey::Digit3 => InputKind::Physical(Code::Digit3),
        ShortcutKey::Digit4 => InputKind::Physical(Code::Digit4),
        ShortcutKey::Digit5 => InputKind::Physical(Code::Digit5),
        ShortcutKey::ArrowUp => InputKind::KeyCode(Named::ArrowUp),
        ShortcutKey::ArrowDown => InputKind::KeyCode(Named::ArrowDown),
    };

    KeyboardBinding {
        target,
        modifiers: iced_modifiers(keys.modifiers()),
        mode_include: SurfaceMode::empty(),
        mode_exclude: excluded,
    }
}

/// The modifiers of the shortcut table as iced writes them.
fn iced_modifiers(modifiers: KeyModifiers) -> Modifiers {
    let mut held = Modifiers::empty();
    if modifiers.ctrl() {
        held |= Modifiers::CTRL;
    }
    if modifiers.alt() {
        held |= Modifiers::ALT;
    }
    if modifiers.logo() {
        held |= Modifiers::LOGO;
    }
    if modifiers.shift() {
        held |= Modifiers::SHIFT;
    }

    held
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_bindings_register_every_shortcut_with_its_keys() {
        for platform in [Platform::Linux, Platform::MacOs] {
            let bindings = bindings_on(platform);

            assert_eq!(bindings.len(), AppShortcut::ALL.len());
            for ((binding, action), shortcut) in
                bindings.iter().zip(AppShortcut::ALL)
            {
                let keys = shortcut.keys_on(platform);

                assert_eq!(binding.modifiers, iced_modifiers(keys.modifiers()));
                assert_eq!(binding.mode_include, SurfaceMode::empty());
                assert_eq!(
                    binding.mode_exclude,
                    screens_left_to_the_program(shortcut)
                );
                assert_eq!(action, &BindingAction::Action(shortcut));
            }
        }
    }

    /// The digit row is matched by its place, because Shift and the layout
    /// change its character; every other key by the character it types.
    #[test]
    fn digit_keys_bind_by_place_and_other_keys_by_character() {
        let digit = ShortcutKeys::new(
            ShortcutKey::Digit2,
            KeyModifiers::NONE.with_ctrl().with_shift(),
        );
        let character = ShortcutKeys::new(
            ShortcutKey::Character('t'),
            KeyModifiers::NONE.with_ctrl().with_shift(),
        );

        assert_eq!(
            keyboard_binding(digit, SurfaceMode::empty()).target,
            InputKind::Physical(Code::Digit2)
        );
        assert_eq!(
            keyboard_binding(character, SurfaceMode::empty()).target,
            InputKind::Char(String::from("t"))
        );
        assert_eq!(
            keyboard_binding(character, SurfaceMode::empty()).modifiers,
            Modifiers::CTRL | Modifiers::SHIFT
        );
    }

    /// A full-screen program has no blocks, so there the block keys reach
    /// the program; every other shortcut belongs to the app on every
    /// screen.
    #[test]
    fn only_the_block_keys_are_left_to_a_full_screen_program() {
        for shortcut in AppShortcut::ALL {
            let is_block_key = matches!(
                shortcut,
                AppShortcut::PreviousBlock | AppShortcut::NextBlock
            );
            let expected = if is_block_key {
                SurfaceMode::ALT_SCREEN
            } else {
                SurfaceMode::empty()
            };

            assert_eq!(
                screens_left_to_the_program(shortcut),
                expected,
                "{shortcut:?}"
            );
        }
    }

    #[test]
    fn arrow_keys_bind_by_name() {
        let up = ShortcutKeys::new(
            ShortcutKey::ArrowUp,
            KeyModifiers::NONE.with_ctrl().with_shift(),
        );
        let down = ShortcutKeys::new(
            ShortcutKey::ArrowDown,
            KeyModifiers::NONE.with_logo(),
        );

        assert_eq!(
            keyboard_binding(up, SurfaceMode::ALT_SCREEN).target,
            InputKind::KeyCode(Named::ArrowUp)
        );
        assert_eq!(
            keyboard_binding(down, SurfaceMode::empty()).target,
            InputKind::KeyCode(Named::ArrowDown)
        );
    }

    #[test]
    fn every_modifier_of_the_table_is_held() {
        let all = KeyModifiers::NONE
            .with_ctrl()
            .with_alt()
            .with_logo()
            .with_shift();

        assert_eq!(iced_modifiers(KeyModifiers::NONE), Modifiers::empty());
        assert_eq!(
            iced_modifiers(all),
            Modifiers::CTRL
                | Modifiers::ALT
                | Modifiers::LOGO
                | Modifiers::SHIFT
        );
    }

    #[test]
    fn terminal_bindings_use_the_current_platform() {
        assert_eq!(terminal_bindings(), bindings_on(Platform::current()));
    }
}

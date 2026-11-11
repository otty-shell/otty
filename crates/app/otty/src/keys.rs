//! Turns an iced key press into the app shortcut it presses.
//!
//! The keys of every shortcut are one table, `AppShortcut::keys_on` in
//! `otty-app-domain`, which also builds the labels. A key press that no
//! widget captured reaches the app keyboard subscription
//! (`iced::keyboard::listen`), and [`shortcut_for_key_press`] finds its
//! shortcut there. `update.rs` then turns the shortcut into its command
//! before the input checks.
//!
//! While a terminal has keyboard focus, the terminal widget matches its own
//! bindings and reports the shortcut instead; the terminal feature builds
//! those bindings from the same table, and `update.rs` turns the report
//! into the command too.

use iced::keyboard::key::{Code, Physical};
use iced::keyboard::{self, Key, Modifiers};
use otty_app_domain::keys::{
    KeyModifiers, Platform, ShortcutKey, ShortcutKeys,
};
use otty_app_domain::shortcuts::AppShortcut;

/// Return the shortcut a key press from the app keyboard subscription
/// stands for.
///
/// It matches keys the way the terminal widget matches its bindings: a
/// digit-row key by its place on the keyboard, any other key by the
/// character it types without modifiers, lower-cased; and exactly the
/// table's modifiers. Auto-repeat presses, key releases and modifier changes
/// return `None`, so a held key runs its shortcut once.
pub(crate) fn shortcut_for_key_press(
    event: &keyboard::Event,
    platform: Platform,
) -> Option<AppShortcut> {
    let keyboard::Event::KeyPressed {
        key,
        physical_key,
        modifiers,
        repeat: false,
        ..
    } = event
    else {
        return None;
    };

    let key = shortcut_key(key, *physical_key)?;
    let pressed = ShortcutKeys::new(key, key_modifiers(*modifiers));

    AppShortcut::ALL
        .into_iter()
        .find(|shortcut| shortcut.keys_on(platform) == pressed)
}

/// The shortcut key of a press: the digit-row keys 1 to 5 by their place,
/// because Shift and the layout change their character (Shift 1 is `!` on
/// a US layout, and the unshifted key is `&` on a French one); any other key
/// by the one character it types, lower-cased. `None` for a named key such
/// as Enter.
fn shortcut_key(key: &Key, physical: Physical) -> Option<ShortcutKey> {
    let digit = match physical {
        Physical::Code(Code::Digit1) => Some(ShortcutKey::Digit1),
        Physical::Code(Code::Digit2) => Some(ShortcutKey::Digit2),
        Physical::Code(Code::Digit3) => Some(ShortcutKey::Digit3),
        Physical::Code(Code::Digit4) => Some(ShortcutKey::Digit4),
        Physical::Code(Code::Digit5) => Some(ShortcutKey::Digit5),
        _ => None,
    };
    if digit.is_some() {
        return digit;
    }

    let Key::Character(text) = key else {
        return None;
    };
    let mut characters = text.chars();
    let character = characters.next()?;
    if characters.next().is_some() {
        return None;
    }

    Some(ShortcutKey::Character(character.to_ascii_lowercase()))
}

/// The held modifiers as the shortcut table writes them.
fn key_modifiers(modifiers: Modifiers) -> KeyModifiers {
    let mut held = KeyModifiers::NONE;
    if modifiers.control() {
        held = held.with_ctrl();
    }
    if modifiers.alt() {
        held = held.with_alt();
    }
    if modifiers.logo() {
        held = held.with_logo();
    }
    if modifiers.shift() {
        held = held.with_shift();
    }

    held
}

#[cfg(test)]
mod tests {
    use iced::keyboard::key::{Code, NativeCode, Physical};
    use iced::keyboard::{Event, Key, Location, Modifiers};

    use super::*;
    use crate::testing::keyboard::{key_press, shortcut_press_on};

    const CTRL_SHIFT: Modifiers = Modifiers::CTRL.union(Modifiers::SHIFT);

    fn character_press(key: &str, modifiers: Modifiers) -> Event {
        key_press(
            Physical::Unidentified(NativeCode::Unidentified),
            key,
            modifiers,
        )
    }

    #[test]
    fn every_shortcut_is_found_from_its_own_key_press() {
        for platform in [Platform::Linux, Platform::MacOs] {
            for shortcut in AppShortcut::ALL {
                let event = shortcut_press_on(shortcut, platform);

                assert_eq!(
                    shortcut_for_key_press(&event, platform),
                    Some(shortcut),
                    "{shortcut:?} on {platform:?}"
                );
            }
        }
    }

    #[test]
    fn upper_case_key_is_matched_like_the_terminal_does() {
        let event = character_press("T", CTRL_SHIFT);

        assert_eq!(
            shortcut_for_key_press(&event, Platform::Linux),
            Some(AppShortcut::NewTerminal)
        );
    }

    #[test]
    fn repeated_press_runs_no_shortcut() {
        let Event::KeyPressed {
            key,
            modified_key,
            physical_key,
            location,
            modifiers,
            text,
            ..
        } = character_press("t", CTRL_SHIFT)
        else {
            panic!("a key press");
        };
        let held = Event::KeyPressed {
            key,
            modified_key,
            physical_key,
            location,
            modifiers,
            text,
            repeat: true,
        };

        assert_eq!(shortcut_for_key_press(&held, Platform::Linux), None);
    }

    #[test]
    fn key_release_and_modifier_change_run_no_shortcut() {
        let release = Event::KeyReleased {
            key: Key::Character("t".into()),
            modified_key: Key::Character("t".into()),
            physical_key: Physical::Unidentified(NativeCode::Unidentified),
            location: Location::Standard,
            modifiers: CTRL_SHIFT,
        };
        let modifiers_changed = Event::ModifiersChanged(CTRL_SHIFT);

        assert_eq!(shortcut_for_key_press(&release, Platform::Linux), None);
        assert_eq!(
            shortcut_for_key_press(&modifiers_changed, Platform::Linux),
            None
        );
    }

    #[test]
    fn shell_keys_run_no_shortcut_on_linux() {
        for key in ["b", "d", "k", "t", "w", "\\", "-"] {
            let event = character_press(key, Modifiers::CTRL);

            assert_eq!(
                shortcut_for_key_press(&event, Platform::Linux),
                None,
                "Ctrl {key} must stay with the shell"
            );
        }
    }

    #[test]
    fn extra_modifier_runs_no_shortcut() {
        let event = character_press("t", CTRL_SHIFT.union(Modifiers::ALT));

        assert_eq!(shortcut_for_key_press(&event, Platform::Linux), None);
    }

    #[test]
    fn named_key_runs_no_shortcut() {
        let event = Event::KeyPressed {
            key: Key::Named(iced::keyboard::key::Named::Enter),
            modified_key: Key::Named(iced::keyboard::key::Named::Enter),
            physical_key: Physical::Unidentified(NativeCode::Unidentified),
            location: Location::Standard,
            modifiers: CTRL_SHIFT,
            text: None,
            repeat: false,
        };

        assert_eq!(shortcut_for_key_press(&event, Platform::Linux), None);
    }

    #[test]
    fn other_platform_keys_run_no_shortcut() {
        let linux_new_terminal = character_press("t", CTRL_SHIFT);

        assert_eq!(
            shortcut_for_key_press(&linux_new_terminal, Platform::MacOs),
            None
        );
    }

    #[test]
    fn a_digit_shortcut_runs_whatever_character_the_layout_types() {
        // Ctrl Shift 1 on a US layout reports "1" or "!", on a French
        // AZERTY layout "&".
        for character in ["1", "!", "&"] {
            let event =
                key_press(Physical::Code(Code::Digit1), character, CTRL_SHIFT);

            assert_eq!(
                shortcut_for_key_press(&event, Platform::Linux),
                Some(AppShortcut::TerminalMode),
                "{character:?}"
            );
        }
    }

    #[test]
    fn the_same_digit_on_another_key_runs_no_mode_shortcut() {
        let keypad = key_press(Physical::Code(Code::Numpad2), "2", CTRL_SHIFT);
        let unknown_key = character_press("2", CTRL_SHIFT);

        assert_eq!(shortcut_for_key_press(&keypad, Platform::Linux), None);
        assert_eq!(shortcut_for_key_press(&unknown_key, Platform::Linux), None);
    }

    #[test]
    fn ctrl_digits_and_ctrl_m_stay_with_the_shell_on_linux() {
        let keys = [
            (Code::Digit1, "1"),
            (Code::Digit2, "2"),
            (Code::Digit3, "3"),
            (Code::Digit4, "4"),
            (Code::Digit5, "5"),
            (Code::KeyM, "m"),
        ];

        for (code, key) in keys {
            let event = key_press(Physical::Code(code), key, Modifiers::CTRL);

            assert_eq!(
                shortcut_for_key_press(&event, Platform::Linux),
                None,
                "Ctrl {key} must stay with the shell"
            );
        }
    }
}

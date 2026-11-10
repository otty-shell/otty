//! Key presses as the keyboard subscription reports them, for tests.

use iced::keyboard::key::{Code, Named, NativeCode, Physical};
use iced::keyboard::{self, Key, Location, Modifiers};
use otty_app_domain::keys::{KeyModifiers, Platform, ShortcutKey};
use otty_app_domain::shortcuts::AppShortcut;

/// The press of `shortcut`'s keys on the current platform, as the keyboard
/// reports it.
pub(crate) fn shortcut_press(shortcut: AppShortcut) -> keyboard::Event {
    shortcut_press_on(shortcut, Platform::current())
}

/// The press of `shortcut`'s keys on `platform`: the physical key for a
/// digit-row key, the character otherwise.
pub(crate) fn shortcut_press_on(
    shortcut: AppShortcut,
    platform: Platform,
) -> keyboard::Event {
    let keys = shortcut.keys_on(platform);
    let (physical_key, text) = match keys.key() {
        ShortcutKey::Character(character) => (
            Physical::Unidentified(NativeCode::Unidentified),
            character.to_string(),
        ),
        ShortcutKey::Digit1 => (Physical::Code(Code::Digit1), "1".into()),
        ShortcutKey::Digit2 => (Physical::Code(Code::Digit2), "2".into()),
        ShortcutKey::Digit3 => (Physical::Code(Code::Digit3), "3".into()),
        ShortcutKey::Digit4 => (Physical::Code(Code::Digit4), "4".into()),
        ShortcutKey::Digit5 => (Physical::Code(Code::Digit5), "5".into()),
    };

    key_press(physical_key, &text, iced_modifiers(keys.modifiers()))
}

/// A press of the physical key `physical_key` that reports `text` with
/// `modifiers` held.
pub(crate) fn key_press(
    physical_key: Physical,
    text: &str,
    modifiers: Modifiers,
) -> keyboard::Event {
    keyboard::Event::KeyPressed {
        key: Key::Character(text.into()),
        modified_key: Key::Character(text.into()),
        physical_key,
        location: Location::Standard,
        modifiers,
        text: None,
        repeat: false,
    }
}

/// A press of Esc, not held.
pub(crate) fn escape_press() -> keyboard::Event {
    keyboard::Event::KeyPressed {
        key: Key::Named(Named::Escape),
        modified_key: Key::Named(Named::Escape),
        physical_key: Physical::Code(Code::Escape),
        location: Location::Standard,
        modifiers: Modifiers::empty(),
        text: None,
        repeat: false,
    }
}

/// The iced modifiers for `modifiers`.
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

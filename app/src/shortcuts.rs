//! The app keymap: every application shortcut and the keys that press it.
//!
//! One table ([`AppShortcut::keys`]) feeds both routes a key press can take:
//!
//! - While a terminal has keyboard focus, the terminal widget matches the key
//!   against [`terminal_bindings`] and reports it as
//!   [`otty_ui_term::Event::BindingActionDispatched`]. The widget marks the
//!   key as captured, so the app keyboard subscription never sees it.
//! - Otherwise the key reaches the app keyboard subscription
//!   (`iced::keyboard::listen`, which only delivers keys no widget captured)
//!   and [`shortcut_for_key_press`] finds the shortcut.
//!
//! Auto-repeat presses run nothing on either route, so a held key runs its
//! shortcut once.

use iced::keyboard::{self, Key, Modifiers};
use otty_ui_kit::components::kbd::shortcut_text;
use otty_ui_term::SurfaceMode;
use otty_ui_term::bindings::{BindingAction, InputKind, KeyboardBinding};

/// An application action that has a keyboard shortcut.
///
/// This is also the action type terminal widgets report back: the `T` of
/// [`otty_ui_term::BindingAction`] and [`otty_ui_term::Event`]. The terminal
/// widget never interprets it, it only carries it from the matched key to
/// the event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum AppShortcut {
    /// Open the command palette.
    CommandPalette,
    /// Show or hide the left panel.
    LeftPanel,
    /// Show or hide the files panel.
    FilesPanel,
    /// Enter or leave focus mode.
    FocusMode,
    /// Open the Settings tab.
    Settings,
    /// Open a new terminal tab.
    NewTerminal,
    /// Close the active tab.
    CloseSession,
    /// Split the focused pane vertically.
    SplitPane,
}

impl AppShortcut {
    /// Every shortcut, in the order of the spec table.
    pub(crate) const ALL: [AppShortcut; 8] = [
        AppShortcut::CommandPalette,
        AppShortcut::LeftPanel,
        AppShortcut::FilesPanel,
        AppShortcut::FocusMode,
        AppShortcut::Settings,
        AppShortcut::NewTerminal,
        AppShortcut::CloseSession,
        AppShortcut::SplitPane,
    ];

    /// Return the keys that press this shortcut on `platform`.
    ///
    /// This is the keymap table. On Linux app shortcuts use Ctrl Shift, so
    /// Ctrl plus a letter, `\` or `-` stays with the shell; on macOS they
    /// use Cmd. Split pane keeps its old keys: Super Shift D on Linux, so
    /// Ctrl D still sends EOF, and Cmd D on macOS.
    pub(crate) fn keys(self, platform: Platform) -> ShortcutKeys {
        let ctrl_shift = Modifiers::CTRL | Modifiers::SHIFT;
        let cmd = Modifiers::LOGO;
        let cmd_shift = Modifiers::LOGO | Modifiers::SHIFT;

        match (self, platform) {
            (AppShortcut::CommandPalette, Platform::Linux) => {
                ShortcutKeys::new("k", ctrl_shift)
            },
            (AppShortcut::CommandPalette, Platform::MacOs) => {
                ShortcutKeys::new("k", cmd)
            },
            (AppShortcut::LeftPanel, Platform::Linux) => {
                ShortcutKeys::new("b", ctrl_shift)
            },
            (AppShortcut::LeftPanel, Platform::MacOs) => {
                ShortcutKeys::new("b", cmd)
            },
            // Not Ctrl Shift E: GNOME's IBus takes it for emoji input.
            (AppShortcut::FilesPanel, Platform::Linux) => {
                ShortcutKeys::new("o", ctrl_shift)
            },
            (AppShortcut::FilesPanel, Platform::MacOs) => {
                ShortcutKeys::new("e", cmd_shift)
            },
            (AppShortcut::FocusMode, Platform::Linux) => {
                ShortcutKeys::new("f", ctrl_shift)
            },
            (AppShortcut::FocusMode, Platform::MacOs) => {
                ShortcutKeys::new("f", cmd_shift)
            },
            (AppShortcut::Settings, Platform::Linux) => {
                ShortcutKeys::new(",", Modifiers::CTRL)
            },
            (AppShortcut::Settings, Platform::MacOs) => {
                ShortcutKeys::new(",", cmd)
            },
            (AppShortcut::NewTerminal, Platform::Linux) => {
                ShortcutKeys::new("t", ctrl_shift)
            },
            (AppShortcut::NewTerminal, Platform::MacOs) => {
                ShortcutKeys::new("t", cmd)
            },
            (AppShortcut::CloseSession, Platform::Linux) => {
                ShortcutKeys::new("w", ctrl_shift)
            },
            (AppShortcut::CloseSession, Platform::MacOs) => {
                ShortcutKeys::new("w", cmd)
            },
            (AppShortcut::SplitPane, Platform::Linux) => {
                ShortcutKeys::new("d", cmd_shift)
            },
            (AppShortcut::SplitPane, Platform::MacOs) => {
                ShortcutKeys::new("d", cmd)
            },
        }
    }

    /// Return the shortcut text for menus and `Kbd` hints on the current
    /// platform, for example "Ctrl Shift T".
    pub(crate) fn label(self) -> String {
        self.label_on(Platform::current())
    }

    /// Return the shortcut text for `platform`, in Linux notation.
    pub(crate) fn label_on(self, platform: Platform) -> String {
        let keys = self.keys(platform);
        shortcut_text(keys.modifiers(), keys.key())
    }
}

/// The platform a shortcut binding is written for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Platform {
    /// Linux and every other system that is not macOS: Ctrl Shift
    /// shortcuts.
    Linux,
    /// macOS: Cmd shortcuts.
    MacOs,
}

impl Platform {
    /// Return the platform this build runs on.
    pub(crate) fn current() -> Self {
        if cfg!(target_os = "macos") {
            Platform::MacOs
        } else {
            Platform::Linux
        }
    }
}

/// A key and the exact modifiers that must be held with it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ShortcutKeys {
    key: &'static str,
    modifiers: Modifiers,
}

impl ShortcutKeys {
    /// The key as the keyboard reports it without modifiers, lower case.
    pub(crate) fn key(&self) -> &'static str {
        self.key
    }

    /// The modifiers that must be held, exactly.
    pub(crate) fn modifiers(&self) -> Modifiers {
        self.modifiers
    }

    fn new(key: &'static str, modifiers: Modifiers) -> Self {
        Self { key, modifiers }
    }
}

/// Return the bindings every terminal widget registers: one per app
/// shortcut, for the current platform.
///
/// They replace the terminal's own binding for the same keys (for example
/// Ctrl Shift T, which the terminal would otherwise send as `^T`).
pub(crate) fn terminal_bindings()
-> Vec<(KeyboardBinding, BindingAction<AppShortcut>)> {
    terminal_bindings_for(Platform::current())
}

/// Return the shortcut a key press from the app keyboard subscription
/// stands for.
///
/// It matches keys the same way the terminal widget matches its bindings:
/// the key without modifiers, lower-cased, and exactly the table's
/// modifiers. Auto-repeat presses, key releases and modifier changes return
/// `None`.
pub(crate) fn shortcut_for_key_press(
    event: &keyboard::Event,
    platform: Platform,
) -> Option<AppShortcut> {
    let keyboard::Event::KeyPressed {
        key: Key::Character(key),
        modifiers,
        repeat: false,
        ..
    } = event
    else {
        return None;
    };

    let key = key.to_ascii_lowercase();
    AppShortcut::ALL.into_iter().find(|shortcut| {
        let keys = shortcut.keys(platform);
        keys.key() == key && keys.modifiers() == *modifiers
    })
}

fn terminal_bindings_for(
    platform: Platform,
) -> Vec<(KeyboardBinding, BindingAction<AppShortcut>)> {
    AppShortcut::ALL
        .into_iter()
        .map(|shortcut| {
            let keys = shortcut.keys(platform);
            let binding = KeyboardBinding {
                target: InputKind::Char(keys.key().to_string()),
                modifiers: keys.modifiers(),
                mode_include: SurfaceMode::empty(),
                mode_exclude: SurfaceMode::empty(),
            };

            (binding, BindingAction::Action(shortcut))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use iced::keyboard::key::{NativeCode, Physical};
    use iced::keyboard::{Event, Key, Location, Modifiers};

    use super::*;

    const CTRL_SHIFT: Modifiers = Modifiers::CTRL.union(Modifiers::SHIFT);

    fn key_press(key: &str, modifiers: Modifiers, repeat: bool) -> Event {
        Event::KeyPressed {
            key: Key::Character(key.into()),
            modified_key: Key::Character(key.into()),
            physical_key: Physical::Unidentified(NativeCode::Unidentified),
            location: Location::Standard,
            modifiers,
            text: None,
            repeat,
        }
    }

    #[test]
    fn every_shortcut_has_a_key_and_a_modifier_on_both_platforms() {
        for platform in [Platform::Linux, Platform::MacOs] {
            for shortcut in AppShortcut::ALL {
                let keys = shortcut.keys(platform);

                assert!(
                    !keys.key().is_empty(),
                    "{shortcut:?} needs a key on {platform:?}"
                );
                assert!(
                    !keys.modifiers().is_empty(),
                    "{shortcut:?} needs a modifier on {platform:?}, or it \
                     would take a plain key from the terminal"
                );
            }
        }
    }

    #[test]
    fn linux_shortcuts_never_take_ctrl_letter_backslash_or_minus_from_the_shell()
     {
        for shortcut in AppShortcut::ALL {
            let keys = shortcut.keys(Platform::Linux);
            let modifiers = keys.modifiers();
            let is_shell_key =
                keys.key().chars().all(|c| c.is_ascii_alphabetic())
                    || keys.key() == "\\"
                    || keys.key() == "-";

            assert!(
                !(modifiers.control() && !modifiers.shift() && is_shell_key),
                "{shortcut:?} takes Ctrl {} from the shell",
                keys.key()
            );
        }
    }

    #[test]
    fn no_two_shortcuts_share_keys_on_the_same_platform() {
        for platform in [Platform::Linux, Platform::MacOs] {
            for (index, shortcut) in AppShortcut::ALL.iter().enumerate() {
                for other in &AppShortcut::ALL[index + 1..] {
                    assert_ne!(
                        shortcut.keys(platform),
                        other.keys(platform),
                        "{shortcut:?} and {other:?} share keys on {platform:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn linux_labels_follow_the_spec_table() {
        let labels: Vec<(AppShortcut, String)> = AppShortcut::ALL
            .into_iter()
            .map(|shortcut| (shortcut, shortcut.label_on(Platform::Linux)))
            .collect();

        assert_eq!(
            labels,
            vec![
                (AppShortcut::CommandPalette, String::from("Ctrl Shift K")),
                (AppShortcut::LeftPanel, String::from("Ctrl Shift B")),
                (AppShortcut::FilesPanel, String::from("Ctrl Shift O")),
                (AppShortcut::FocusMode, String::from("Ctrl Shift F")),
                (AppShortcut::Settings, String::from("Ctrl ,")),
                (AppShortcut::NewTerminal, String::from("Ctrl Shift T")),
                (AppShortcut::CloseSession, String::from("Ctrl Shift W")),
                (AppShortcut::SplitPane, String::from("Shift Super D")),
            ]
        );
    }

    #[test]
    fn macos_labels_follow_the_spec_table() {
        // `shortcut_text` writes Cmd in Linux notation, as Super.
        let labels: Vec<(AppShortcut, String)> = AppShortcut::ALL
            .into_iter()
            .map(|shortcut| (shortcut, shortcut.label_on(Platform::MacOs)))
            .collect();

        assert_eq!(
            labels,
            vec![
                (AppShortcut::CommandPalette, String::from("Super K")),
                (AppShortcut::LeftPanel, String::from("Super B")),
                (AppShortcut::FilesPanel, String::from("Shift Super E")),
                (AppShortcut::FocusMode, String::from("Shift Super F")),
                (AppShortcut::Settings, String::from("Super ,")),
                (AppShortcut::NewTerminal, String::from("Super T")),
                (AppShortcut::CloseSession, String::from("Super W")),
                (AppShortcut::SplitPane, String::from("Super D")),
            ]
        );
    }

    #[test]
    fn labels_are_built_from_the_bindings() {
        for platform in [Platform::Linux, Platform::MacOs] {
            for shortcut in AppShortcut::ALL {
                let keys = shortcut.keys(platform);

                assert_eq!(
                    shortcut.label_on(platform),
                    shortcut_text(keys.modifiers(), keys.key())
                );
            }
        }
    }

    #[test]
    fn label_uses_the_current_platform() {
        let current = if cfg!(target_os = "macos") {
            Platform::MacOs
        } else {
            Platform::Linux
        };

        assert_eq!(Platform::current(), current);
        for shortcut in AppShortcut::ALL {
            assert_eq!(shortcut.label(), shortcut.label_on(current));
        }
    }

    #[test]
    fn terminal_bindings_register_every_shortcut_with_its_keys() {
        for platform in [Platform::Linux, Platform::MacOs] {
            let bindings = terminal_bindings_for(platform);

            assert_eq!(bindings.len(), AppShortcut::ALL.len());
            for ((binding, action), shortcut) in
                bindings.iter().zip(AppShortcut::ALL)
            {
                let keys = shortcut.keys(platform);

                assert_eq!(
                    binding.target,
                    InputKind::Char(keys.key().to_string())
                );
                assert_eq!(binding.modifiers, keys.modifiers());
                assert_eq!(binding.mode_include, SurfaceMode::empty());
                assert_eq!(binding.mode_exclude, SurfaceMode::empty());
                assert_eq!(action, &BindingAction::Action(shortcut));
            }
        }
    }

    #[test]
    fn terminal_bindings_use_the_current_platform() {
        assert_eq!(
            terminal_bindings(),
            terminal_bindings_for(Platform::current())
        );
    }

    #[test]
    fn every_shortcut_is_found_from_its_own_key_press() {
        for platform in [Platform::Linux, Platform::MacOs] {
            for shortcut in AppShortcut::ALL {
                let keys = shortcut.keys(platform);
                let event = key_press(keys.key(), keys.modifiers(), false);

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
        let event = key_press("T", CTRL_SHIFT, false);

        assert_eq!(
            shortcut_for_key_press(&event, Platform::Linux),
            Some(AppShortcut::NewTerminal)
        );
    }

    #[test]
    fn repeated_press_runs_no_shortcut() {
        let event = key_press("t", CTRL_SHIFT, true);

        assert_eq!(shortcut_for_key_press(&event, Platform::Linux), None);
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
        for key in ["b", "k", "t", "w", "\\", "-"] {
            let event = key_press(key, Modifiers::CTRL, false);

            assert_eq!(
                shortcut_for_key_press(&event, Platform::Linux),
                None,
                "Ctrl {key} must stay with the shell"
            );
        }
    }

    #[test]
    fn extra_modifier_runs_no_shortcut() {
        let event = key_press("t", CTRL_SHIFT.union(Modifiers::ALT), false);

        assert_eq!(shortcut_for_key_press(&event, Platform::Linux), None);
    }

    #[test]
    fn named_key_runs_no_shortcut() {
        let event = Event::KeyPressed {
            key: Key::Named(keyboard::key::Named::Enter),
            modified_key: Key::Named(keyboard::key::Named::Enter),
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
        let linux_new_terminal = key_press("t", CTRL_SHIFT, false);

        assert_eq!(
            shortcut_for_key_press(&linux_new_terminal, Platform::MacOs),
            None
        );
    }
}

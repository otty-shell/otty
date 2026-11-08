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
//!
//! Most keys are matched by the character they type without modifiers. The
//! digit row of the mode shortcuts is matched by physical key instead,
//! because Shift and the layout change its character (Shift 1 is `!` on a
//! US layout, and the unshifted key is `&` on a French one).

use iced::keyboard::key::{Code, Physical};
use iced::keyboard::{self, Key, Modifiers};
use otty_ui_kit::components::kbd::shortcut_text;
use otty_ui_term::SurfaceMode;
use otty_ui_term::bindings::{BindingAction, InputKind, KeyboardBinding};

use crate::commands::AppCommand;

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
    /// Switch to Terminal mode, the first switcher tab.
    TerminalMode,
    /// Switch to the first pinned mode, if any.
    PinnedMode1,
    /// Switch to the second pinned mode, if any.
    PinnedMode2,
    /// Switch to the third pinned mode, if any.
    PinnedMode3,
    /// Switch to the fourth pinned mode, if any.
    PinnedMode4,
    /// Open or close the All modes menu.
    AllModesMenu,
}

impl AppShortcut {
    /// Every shortcut, in the order of the spec tables.
    pub(crate) const ALL: [AppShortcut; 14] = [
        AppShortcut::CommandPalette,
        AppShortcut::LeftPanel,
        AppShortcut::FilesPanel,
        AppShortcut::FocusMode,
        AppShortcut::Settings,
        AppShortcut::NewTerminal,
        AppShortcut::CloseSession,
        AppShortcut::SplitPane,
        AppShortcut::TerminalMode,
        AppShortcut::PinnedMode1,
        AppShortcut::PinnedMode2,
        AppShortcut::PinnedMode3,
        AppShortcut::PinnedMode4,
        AppShortcut::AllModesMenu,
    ];

    /// The mode shortcuts in switcher order: shortcut N selects the mode
    /// at position N of Terminal followed by the pinned modes.
    const MODE_SHORTCUTS: [AppShortcut; 5] = [
        AppShortcut::TerminalMode,
        AppShortcut::PinnedMode1,
        AppShortcut::PinnedMode2,
        AppShortcut::PinnedMode3,
        AppShortcut::PinnedMode4,
    ];

    /// Return the keys that press this shortcut on `platform`.
    ///
    /// This is the keymap table. On Linux app shortcuts use Ctrl Shift, so
    /// Ctrl plus a letter, `\` or `-` stays with the shell; on macOS they
    /// use Cmd. Split pane keeps its old keys: Super Shift D on Linux, so
    /// Ctrl D still sends EOF, and Cmd D on macOS. The mode digits are
    /// physical keys, so Ctrl 1–5 and Ctrl M without Shift stay with the
    /// shell too.
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
            (AppShortcut::TerminalMode, Platform::Linux) => {
                ShortcutKeys::physical(Code::Digit1, "1", ctrl_shift)
            },
            (AppShortcut::TerminalMode, Platform::MacOs) => {
                ShortcutKeys::physical(Code::Digit1, "1", cmd)
            },
            (AppShortcut::PinnedMode1, Platform::Linux) => {
                ShortcutKeys::physical(Code::Digit2, "2", ctrl_shift)
            },
            (AppShortcut::PinnedMode1, Platform::MacOs) => {
                ShortcutKeys::physical(Code::Digit2, "2", cmd)
            },
            (AppShortcut::PinnedMode2, Platform::Linux) => {
                ShortcutKeys::physical(Code::Digit3, "3", ctrl_shift)
            },
            (AppShortcut::PinnedMode2, Platform::MacOs) => {
                ShortcutKeys::physical(Code::Digit3, "3", cmd)
            },
            (AppShortcut::PinnedMode3, Platform::Linux) => {
                ShortcutKeys::physical(Code::Digit4, "4", ctrl_shift)
            },
            (AppShortcut::PinnedMode3, Platform::MacOs) => {
                ShortcutKeys::physical(Code::Digit4, "4", cmd)
            },
            (AppShortcut::PinnedMode4, Platform::Linux) => {
                ShortcutKeys::physical(Code::Digit5, "5", ctrl_shift)
            },
            (AppShortcut::PinnedMode4, Platform::MacOs) => {
                ShortcutKeys::physical(Code::Digit5, "5", cmd)
            },
            (AppShortcut::AllModesMenu, Platform::Linux) => {
                ShortcutKeys::new("m", ctrl_shift)
            },
            (AppShortcut::AllModesMenu, Platform::MacOs) => {
                ShortcutKeys::new("m", cmd_shift)
            },
        }
    }

    /// Return the switcher position this shortcut selects: 0 for Terminal
    /// mode, N for pinned mode N, `None` for every other shortcut.
    pub(crate) fn mode_position(self) -> Option<usize> {
        Self::MODE_SHORTCUTS
            .iter()
            .position(|shortcut| *shortcut == self)
    }

    /// Return the shortcut that selects the mode at switcher `position`
    /// (Terminal, then the pinned modes); `None` past pinned mode 4.
    pub(crate) fn for_mode_position(position: usize) -> Option<AppShortcut> {
        Self::MODE_SHORTCUTS.get(position).copied()
    }

    /// Return the command this shortcut asks for.
    pub(crate) fn command(self) -> AppCommand {
        match self {
            Self::CommandPalette => AppCommand::OpenPalette,
            Self::LeftPanel => AppCommand::ToggleLeftPanel,
            Self::FilesPanel => AppCommand::ToggleFilesPanel,
            Self::FocusMode => AppCommand::ToggleFocusMode,
            Self::Settings => AppCommand::OpenSettings,
            Self::NewTerminal => AppCommand::NewTerminal,
            Self::CloseSession => AppCommand::CloseSession,
            Self::SplitPane => AppCommand::SplitPane,
            Self::TerminalMode => AppCommand::SelectModeAt { position: 0 },
            Self::PinnedMode1 => AppCommand::SelectModeAt { position: 1 },
            Self::PinnedMode2 => AppCommand::SelectModeAt { position: 2 },
            Self::PinnedMode3 => AppCommand::SelectModeAt { position: 3 },
            Self::PinnedMode4 => AppCommand::SelectModeAt { position: 4 },
            Self::AllModesMenu => AppCommand::ToggleModesMenu,
        }
    }

    /// Return the shortcut text for menus and `Kbd` hints on the current
    /// platform, for example "Ctrl Shift T".
    pub(crate) fn label(self) -> String {
        self.label_on(Platform::current())
    }

    /// Return the shortcut text for `platform`: the Logo key reads "Super" on
    /// Linux and "Cmd" on macOS.
    pub(crate) fn label_on(self, platform: Platform) -> String {
        let keys = self.keys(platform);
        shortcut_text(keys.modifiers(), keys.key(), platform.logo_name())
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
    /// Return how this platform's labels write the Logo key.
    fn logo_name(self) -> &'static str {
        match self {
            Platform::MacOs => "Cmd",
            _ => "Super",
        }
    }

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
    code: Option<Code>,
    modifiers: Modifiers,
}

impl ShortcutKeys {
    /// The key as labels show it. For a key matched by character, it is
    /// also the character the keyboard reports without modifiers, lower
    /// case.
    pub(crate) fn key(&self) -> &'static str {
        self.key
    }

    /// The physical key the shortcut is matched by, whatever character it
    /// types; `None` for a key matched by its character.
    pub(crate) fn code(&self) -> Option<Code> {
        self.code
    }

    /// The modifiers that must be held, exactly.
    pub(crate) fn modifiers(&self) -> Modifiers {
        self.modifiers
    }

    /// A key matched by the character it types without modifiers.
    fn new(key: &'static str, modifiers: Modifiers) -> Self {
        Self {
            key,
            code: None,
            modifiers,
        }
    }

    /// A key matched by its place on the keyboard, labelled `label`.
    fn physical(code: Code, label: &'static str, modifiers: Modifiers) -> Self {
        Self {
            key: label,
            code: Some(code),
            modifiers,
        }
    }

    /// Return whether a press of `key` (without modifiers) on the physical
    /// key `physical` with exactly `modifiers` held is this shortcut.
    fn is_pressed_by(
        &self,
        key: &Key,
        physical: Physical,
        modifiers: Modifiers,
    ) -> bool {
        let is_same_key = match self.code {
            Some(code) => physical == Physical::Code(code),
            None => match key {
                Key::Character(key) => key.to_ascii_lowercase() == self.key,
                _ => false,
            },
        };

        is_same_key && modifiers == self.modifiers
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
/// the physical key for the mode digits, otherwise the key without
/// modifiers, lower-cased; and exactly the table's modifiers. Auto-repeat
/// presses, key releases and modifier changes return `None`.
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

    AppShortcut::ALL.into_iter().find(|shortcut| {
        shortcut
            .keys(platform)
            .is_pressed_by(key, *physical_key, *modifiers)
    })
}

fn terminal_bindings_for(
    platform: Platform,
) -> Vec<(KeyboardBinding, BindingAction<AppShortcut>)> {
    AppShortcut::ALL
        .into_iter()
        .map(|shortcut| {
            let keys = shortcut.keys(platform);
            let target = match keys.code() {
                Some(code) => InputKind::Physical(code),
                None => InputKind::Char(keys.key().to_string()),
            };
            let binding = KeyboardBinding {
                target,
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
    use iced::keyboard::key::{Code, NativeCode, Physical};
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

    /// Build a press of the physical key `physical` that reports `key`, as
    /// the layout and the held modifiers make it.
    fn physical_key_press(
        physical: Physical,
        key: &str,
        modifiers: Modifiers,
    ) -> Event {
        Event::KeyPressed {
            key: Key::Character(key.into()),
            modified_key: Key::Character(key.into()),
            physical_key: physical,
            location: Location::Standard,
            modifiers,
            text: None,
            repeat: false,
        }
    }

    /// Build the press the keymap table describes for `keys`: the physical
    /// key when it is matched by place, the character otherwise.
    fn own_key_press(keys: ShortcutKeys) -> Event {
        let physical = match keys.code() {
            Some(code) => Physical::Code(code),
            None => Physical::Unidentified(NativeCode::Unidentified),
        };

        physical_key_press(physical, keys.key(), keys.modifiers())
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
    fn linux_shortcuts_never_take_ctrl_letter_digit_backslash_or_minus_from_the_shell()
     {
        for shortcut in AppShortcut::ALL {
            let keys = shortcut.keys(Platform::Linux);
            let modifiers = keys.modifiers();
            let is_shell_key =
                keys.key().chars().all(|c| c.is_ascii_alphanumeric())
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
                    let keys = shortcut.keys(platform);
                    let other_keys = other.keys(platform);

                    assert_ne!(
                        (keys.key(), keys.modifiers()),
                        (other_keys.key(), other_keys.modifiers()),
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
                (AppShortcut::SplitPane, String::from("Super Shift D")),
                (AppShortcut::TerminalMode, String::from("Ctrl Shift 1")),
                (AppShortcut::PinnedMode1, String::from("Ctrl Shift 2")),
                (AppShortcut::PinnedMode2, String::from("Ctrl Shift 3")),
                (AppShortcut::PinnedMode3, String::from("Ctrl Shift 4")),
                (AppShortcut::PinnedMode4, String::from("Ctrl Shift 5")),
                (AppShortcut::AllModesMenu, String::from("Ctrl Shift M")),
            ]
        );
    }

    #[test]
    fn macos_labels_follow_the_spec_table() {
        let labels: Vec<(AppShortcut, String)> = AppShortcut::ALL
            .into_iter()
            .map(|shortcut| (shortcut, shortcut.label_on(Platform::MacOs)))
            .collect();

        assert_eq!(
            labels,
            vec![
                (AppShortcut::CommandPalette, String::from("Cmd K")),
                (AppShortcut::LeftPanel, String::from("Cmd B")),
                (AppShortcut::FilesPanel, String::from("Cmd Shift E")),
                (AppShortcut::FocusMode, String::from("Cmd Shift F")),
                (AppShortcut::Settings, String::from("Cmd ,")),
                (AppShortcut::NewTerminal, String::from("Cmd T")),
                (AppShortcut::CloseSession, String::from("Cmd W")),
                (AppShortcut::SplitPane, String::from("Cmd D")),
                (AppShortcut::TerminalMode, String::from("Cmd 1")),
                (AppShortcut::PinnedMode1, String::from("Cmd 2")),
                (AppShortcut::PinnedMode2, String::from("Cmd 3")),
                (AppShortcut::PinnedMode3, String::from("Cmd 4")),
                (AppShortcut::PinnedMode4, String::from("Cmd 5")),
                (AppShortcut::AllModesMenu, String::from("Cmd Shift M")),
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
                    shortcut_text(
                        keys.modifiers(),
                        keys.key(),
                        platform.logo_name()
                    )
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
                let target = match keys.code() {
                    Some(code) => InputKind::Physical(code),
                    None => InputKind::Char(keys.key().to_string()),
                };

                assert_eq!(binding.target, target, "{shortcut:?}");
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
                let event = own_key_press(shortcut.keys(platform));

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

    #[test]
    fn mode_digits_are_matched_by_physical_key_on_both_platforms() {
        let digits = [
            (AppShortcut::TerminalMode, Code::Digit1, "1"),
            (AppShortcut::PinnedMode1, Code::Digit2, "2"),
            (AppShortcut::PinnedMode2, Code::Digit3, "3"),
            (AppShortcut::PinnedMode3, Code::Digit4, "4"),
            (AppShortcut::PinnedMode4, Code::Digit5, "5"),
        ];

        for platform in [Platform::Linux, Platform::MacOs] {
            for (shortcut, code, label) in digits {
                let keys = shortcut.keys(platform);

                assert_eq!(keys.code(), Some(code), "{shortcut:?}");
                assert_eq!(keys.key(), label, "{shortcut:?}");
            }
            assert_eq!(AppShortcut::AllModesMenu.keys(platform).code(), None);
        }
    }

    #[test]
    fn a_digit_shortcut_runs_whatever_character_the_layout_types() {
        // Ctrl Shift 1 on a US layout reports "1" or "!", on a French
        // AZERTY layout "&".
        for character in ["1", "!", "&"] {
            let event = physical_key_press(
                Physical::Code(Code::Digit1),
                character,
                CTRL_SHIFT,
            );

            assert_eq!(
                shortcut_for_key_press(&event, Platform::Linux),
                Some(AppShortcut::TerminalMode),
                "{character:?}"
            );
        }
    }

    #[test]
    fn the_same_digit_on_another_key_runs_no_mode_shortcut() {
        let keypad =
            physical_key_press(Physical::Code(Code::Numpad2), "2", CTRL_SHIFT);
        let unknown_key = key_press("2", CTRL_SHIFT, false);

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
            let event =
                physical_key_press(Physical::Code(code), key, Modifiers::CTRL);

            assert_eq!(
                shortcut_for_key_press(&event, Platform::Linux),
                None,
                "Ctrl {key} must stay with the shell"
            );
        }
    }

    #[test]
    fn mode_shortcuts_select_the_switcher_positions_in_order() {
        let positions: Vec<Option<usize>> = AppShortcut::ALL
            .into_iter()
            .map(AppShortcut::mode_position)
            .collect();

        assert_eq!(
            positions,
            vec![
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                Some(0),
                Some(1),
                Some(2),
                Some(3),
                Some(4),
                None,
            ]
        );
    }

    #[test]
    fn every_switcher_position_up_to_four_has_its_shortcut() {
        for position in 0..=4 {
            let shortcut = AppShortcut::for_mode_position(position)
                .expect("positions 0 to 4 have a shortcut");

            assert_eq!(shortcut.mode_position(), Some(position));
        }
        assert_eq!(AppShortcut::for_mode_position(5), None);
    }

    #[test]
    fn every_shortcut_asks_for_its_command() {
        let expected = [
            (AppShortcut::CommandPalette, AppCommand::OpenPalette),
            (AppShortcut::LeftPanel, AppCommand::ToggleLeftPanel),
            (AppShortcut::FilesPanel, AppCommand::ToggleFilesPanel),
            (AppShortcut::FocusMode, AppCommand::ToggleFocusMode),
            (AppShortcut::Settings, AppCommand::OpenSettings),
            (AppShortcut::NewTerminal, AppCommand::NewTerminal),
            (AppShortcut::CloseSession, AppCommand::CloseSession),
            (AppShortcut::SplitPane, AppCommand::SplitPane),
            (
                AppShortcut::TerminalMode,
                AppCommand::SelectModeAt { position: 0 },
            ),
            (
                AppShortcut::PinnedMode1,
                AppCommand::SelectModeAt { position: 1 },
            ),
            (
                AppShortcut::PinnedMode2,
                AppCommand::SelectModeAt { position: 2 },
            ),
            (
                AppShortcut::PinnedMode3,
                AppCommand::SelectModeAt { position: 3 },
            ),
            (
                AppShortcut::PinnedMode4,
                AppCommand::SelectModeAt { position: 4 },
            ),
            (AppShortcut::AllModesMenu, AppCommand::ToggleModesMenu),
        ];

        assert_eq!(expected.len(), AppShortcut::ALL.len());
        for (shortcut, command) in expected {
            assert_eq!(shortcut.command(), command, "{shortcut:?}");
        }
    }
}

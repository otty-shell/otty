//! The app keymap: every application shortcut, the keys that press it, the
//! command it asks for and the label menus and `Kbd` hints show for it.
//!
//! [`AppShortcut::keys_on`] is the one table of keys. The labels are built
//! from it, and the binary matches key presses against it: it turns an iced
//! key press into [`ShortcutKeys`] and looks the shortcut up here.
//!
//! Most keys are matched by the character they type without modifiers. The
//! digit row of the mode shortcuts is matched by physical key instead,
//! because Shift and the layout change its character (Shift 1 is `!` on a
//! US layout, and the unshifted key is `&` on a French one).

use crate::commands::AppCommand;
use crate::keys::{KeyModifiers, Platform, ShortcutKey, ShortcutKeys};

/// An application action that has a keyboard shortcut.
///
/// This is also the action type terminal widgets report back. The terminal
/// widget never interprets it, it only carries it from the matched key to
/// the event.
///
/// # Examples
///
/// ```
/// use otty_app_domain::commands::AppCommand;
/// use otty_app_domain::keys::Platform;
/// use otty_app_domain::shortcuts::AppShortcut;
///
/// let shortcut = AppShortcut::NewTerminal;
///
/// assert_eq!(shortcut.command(), AppCommand::NewTerminal);
/// assert_eq!(shortcut.label_on(Platform::Linux), "Ctrl Shift T");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AppShortcut {
    /// Open the command palette.
    CommandPalette,
    /// Show or hide the left panel.
    LeftPanel,
    /// Show or hide the files panel.
    FilesPanel,
    /// Enter or leave focus mode.
    FocusMode,
    /// Open the Settings popup over the active mode.
    Settings,
    /// Open a new terminal tab.
    NewTerminal,
    /// Close the active tab.
    CloseSession,
    /// Split the focused pane to the right.
    SplitRight,
    /// Split the focused pane down.
    SplitDown,
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
    pub const ALL: [AppShortcut; 15] = [
        AppShortcut::CommandPalette,
        AppShortcut::LeftPanel,
        AppShortcut::FilesPanel,
        AppShortcut::FocusMode,
        AppShortcut::Settings,
        AppShortcut::NewTerminal,
        AppShortcut::CloseSession,
        AppShortcut::SplitRight,
        AppShortcut::SplitDown,
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
    /// use Cmd. Split right keeps its old keys: Super Shift D on Linux and
    /// Cmd D on macOS. Split down is Ctrl Shift D on Linux, so plain Ctrl D
    /// still sends EOF, and Cmd Shift D on macOS. The mode digits are
    /// physical keys, so Ctrl 1–5 and Ctrl M without Shift stay with the
    /// shell too.
    pub fn keys_on(self, platform: Platform) -> ShortcutKeys {
        let ctrl = KeyModifiers::NONE.with_ctrl();
        let ctrl_shift = ctrl.with_shift();
        let cmd = KeyModifiers::NONE.with_logo();
        let cmd_shift = cmd.with_shift();
        let character = ShortcutKey::Character;

        let (key, modifiers) = match (self, platform) {
            (Self::CommandPalette, Platform::Linux) => {
                (character('k'), ctrl_shift)
            },
            (Self::CommandPalette, Platform::MacOs) => (character('k'), cmd),
            (Self::LeftPanel, Platform::Linux) => (character('b'), ctrl_shift),
            (Self::LeftPanel, Platform::MacOs) => (character('b'), cmd),
            // Not Ctrl Shift E: GNOME's IBus takes it for emoji input.
            (Self::FilesPanel, Platform::Linux) => (character('o'), ctrl_shift),
            (Self::FilesPanel, Platform::MacOs) => (character('e'), cmd_shift),
            (Self::FocusMode, Platform::Linux) => (character('f'), ctrl_shift),
            (Self::FocusMode, Platform::MacOs) => (character('f'), cmd_shift),
            (Self::Settings, Platform::Linux) => (character(','), ctrl),
            (Self::Settings, Platform::MacOs) => (character(','), cmd),
            (Self::NewTerminal, Platform::Linux) => {
                (character('t'), ctrl_shift)
            },
            (Self::NewTerminal, Platform::MacOs) => (character('t'), cmd),
            (Self::CloseSession, Platform::Linux) => {
                (character('w'), ctrl_shift)
            },
            (Self::CloseSession, Platform::MacOs) => (character('w'), cmd),
            (Self::SplitRight, Platform::Linux) => (character('d'), cmd_shift),
            (Self::SplitRight, Platform::MacOs) => (character('d'), cmd),
            (Self::SplitDown, Platform::Linux) => (character('d'), ctrl_shift),
            (Self::SplitDown, Platform::MacOs) => (character('d'), cmd_shift),
            (Self::TerminalMode, Platform::Linux) => {
                (ShortcutKey::Digit1, ctrl_shift)
            },
            (Self::TerminalMode, Platform::MacOs) => (ShortcutKey::Digit1, cmd),
            (Self::PinnedMode1, Platform::Linux) => {
                (ShortcutKey::Digit2, ctrl_shift)
            },
            (Self::PinnedMode1, Platform::MacOs) => (ShortcutKey::Digit2, cmd),
            (Self::PinnedMode2, Platform::Linux) => {
                (ShortcutKey::Digit3, ctrl_shift)
            },
            (Self::PinnedMode2, Platform::MacOs) => (ShortcutKey::Digit3, cmd),
            (Self::PinnedMode3, Platform::Linux) => {
                (ShortcutKey::Digit4, ctrl_shift)
            },
            (Self::PinnedMode3, Platform::MacOs) => (ShortcutKey::Digit4, cmd),
            (Self::PinnedMode4, Platform::Linux) => {
                (ShortcutKey::Digit5, ctrl_shift)
            },
            (Self::PinnedMode4, Platform::MacOs) => (ShortcutKey::Digit5, cmd),
            (Self::AllModesMenu, Platform::Linux) => {
                (character('m'), ctrl_shift)
            },
            (Self::AllModesMenu, Platform::MacOs) => {
                (character('m'), cmd_shift)
            },
        };

        ShortcutKeys::new(key, modifiers)
    }

    /// Return the shortcut that selects the mode at switcher `position`
    /// (Terminal, then the pinned modes); `None` past pinned mode 4.
    pub fn for_mode_position(position: usize) -> Option<AppShortcut> {
        Self::MODE_SHORTCUTS.get(position).copied()
    }

    /// Return the command this shortcut asks for.
    pub fn command(self) -> AppCommand {
        match self {
            Self::CommandPalette => AppCommand::OpenPalette,
            Self::LeftPanel => AppCommand::ToggleLeftPanel,
            Self::FilesPanel => AppCommand::ToggleFilesPanel,
            Self::FocusMode => AppCommand::ToggleFocusMode,
            Self::Settings => AppCommand::OpenSettings,
            Self::NewTerminal => AppCommand::NewTerminal,
            Self::CloseSession => AppCommand::CloseSession,
            Self::SplitRight => AppCommand::SplitRight,
            Self::SplitDown => AppCommand::SplitDown,
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
    pub fn label(self) -> String {
        self.label_on(Platform::current())
    }

    /// Return the shortcut text for `platform`, built from its keys: the
    /// Logo key reads "Super" on Linux and "Cmd" on macOS.
    pub fn label_on(self, platform: Platform) -> String {
        self.keys_on(platform).label_on(platform)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_shortcut_has_a_modifier_on_both_platforms() {
        for platform in [Platform::Linux, Platform::MacOs] {
            for shortcut in AppShortcut::ALL {
                let keys = shortcut.keys_on(platform);

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
            let keys = shortcut.keys_on(Platform::Linux);
            let modifiers = keys.modifiers();
            let is_shell_key = match keys.key() {
                ShortcutKey::Character(key) => {
                    key.is_ascii_alphanumeric() || key == '\\' || key == '-'
                },
                _ => true,
            };

            assert!(
                !(modifiers.ctrl() && !modifiers.shift() && is_shell_key),
                "{shortcut:?} takes Ctrl {:?} from the shell",
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
                        shortcut.keys_on(platform),
                        other.keys_on(platform),
                        "{shortcut:?} and {other:?} share keys on {platform:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn no_two_shortcuts_share_a_label_on_the_same_platform() {
        for platform in [Platform::Linux, Platform::MacOs] {
            for (index, shortcut) in AppShortcut::ALL.iter().enumerate() {
                for other in &AppShortcut::ALL[index + 1..] {
                    assert_ne!(
                        shortcut.label_on(platform),
                        other.label_on(platform),
                        "{shortcut:?} and {other:?} share a label on \
                         {platform:?}"
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
                (AppShortcut::SplitRight, String::from("Super Shift D")),
                (AppShortcut::SplitDown, String::from("Ctrl Shift D")),
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
                (AppShortcut::SplitRight, String::from("Cmd D")),
                (AppShortcut::SplitDown, String::from("Cmd Shift D")),
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
    fn labels_are_built_from_the_keys() {
        for platform in [Platform::Linux, Platform::MacOs] {
            for shortcut in AppShortcut::ALL {
                assert_eq!(
                    shortcut.label_on(platform),
                    shortcut.keys_on(platform).label_on(platform),
                    "{shortcut:?} on {platform:?}"
                );
            }
        }
    }

    #[test]
    fn label_uses_the_current_platform() {
        for shortcut in AppShortcut::ALL {
            assert_eq!(
                shortcut.label(),
                shortcut.label_on(Platform::current())
            );
        }
    }

    #[test]
    fn mode_digits_are_matched_by_physical_key_on_both_platforms() {
        let digits = [
            (AppShortcut::TerminalMode, ShortcutKey::Digit1),
            (AppShortcut::PinnedMode1, ShortcutKey::Digit2),
            (AppShortcut::PinnedMode2, ShortcutKey::Digit3),
            (AppShortcut::PinnedMode3, ShortcutKey::Digit4),
            (AppShortcut::PinnedMode4, ShortcutKey::Digit5),
        ];

        for platform in [Platform::Linux, Platform::MacOs] {
            for (shortcut, key) in digits {
                assert_eq!(
                    shortcut.keys_on(platform).key(),
                    key,
                    "{shortcut:?}"
                );
            }
            assert_eq!(
                AppShortcut::AllModesMenu.keys_on(platform).key(),
                ShortcutKey::Character('m')
            );
        }
    }

    #[test]
    fn linux_mode_digits_and_all_modes_menu_need_ctrl_shift() {
        let ctrl_shift = KeyModifiers::NONE.with_ctrl().with_shift();

        for shortcut in [
            AppShortcut::TerminalMode,
            AppShortcut::PinnedMode1,
            AppShortcut::PinnedMode2,
            AppShortcut::PinnedMode3,
            AppShortcut::PinnedMode4,
            AppShortcut::AllModesMenu,
        ] {
            assert_eq!(
                shortcut.keys_on(Platform::Linux).modifiers(),
                ctrl_shift,
                "{shortcut:?}: Ctrl alone must stay with the shell"
            );
        }
    }

    #[test]
    fn every_switcher_position_up_to_four_has_its_shortcut() {
        for position in 0..=4 {
            let shortcut = AppShortcut::for_mode_position(position)
                .expect("positions 0 to 4 have a shortcut");

            // The label a switcher tab shows is the key of the command that
            // selects it.
            assert_eq!(
                shortcut.command(),
                AppCommand::SelectModeAt { position }
            );
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
            (AppShortcut::SplitRight, AppCommand::SplitRight),
            (AppShortcut::SplitDown, AppCommand::SplitDown),
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

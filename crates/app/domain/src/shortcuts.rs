//! The app shortcuts: their names, the command each asks for and the label
//! menus and `Kbd` hints show for it.
//!
//! The keys that press a shortcut and the matching of iced key presses stay
//! with the binary, which knows iced. The labels here must name those keys:
//! the binary checks that the label of every shortcut is the text of its
//! keys.

use crate::commands::AppCommand;

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
/// use otty_app_domain::shortcuts::{AppShortcut, Platform};
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
    pub fn label(self) -> &'static str {
        self.label_on(Platform::current())
    }

    /// Return the shortcut text for `platform`: the Logo key reads "Super" on
    /// Linux and "Cmd" on macOS.
    ///
    /// On Linux app shortcuts use Ctrl Shift, so Ctrl plus a letter, `\` or
    /// `-` stays with the shell; on macOS they use Cmd. Split right keeps its
    /// old keys: Super Shift D on Linux and Cmd D on macOS. Split down is
    /// Ctrl Shift D on Linux, so plain Ctrl D still sends EOF, and Cmd Shift
    /// D on macOS.
    pub fn label_on(self, platform: Platform) -> &'static str {
        match (self, platform) {
            (Self::CommandPalette, Platform::Linux) => "Ctrl Shift K",
            (Self::CommandPalette, Platform::MacOs) => "Cmd K",
            (Self::LeftPanel, Platform::Linux) => "Ctrl Shift B",
            (Self::LeftPanel, Platform::MacOs) => "Cmd B",
            // Not Ctrl Shift E: GNOME's IBus takes it for emoji input.
            (Self::FilesPanel, Platform::Linux) => "Ctrl Shift O",
            (Self::FilesPanel, Platform::MacOs) => "Cmd Shift E",
            (Self::FocusMode, Platform::Linux) => "Ctrl Shift F",
            (Self::FocusMode, Platform::MacOs) => "Cmd Shift F",
            (Self::Settings, Platform::Linux) => "Ctrl ,",
            (Self::Settings, Platform::MacOs) => "Cmd ,",
            (Self::NewTerminal, Platform::Linux) => "Ctrl Shift T",
            (Self::NewTerminal, Platform::MacOs) => "Cmd T",
            (Self::CloseSession, Platform::Linux) => "Ctrl Shift W",
            (Self::CloseSession, Platform::MacOs) => "Cmd W",
            (Self::SplitRight, Platform::Linux) => "Super Shift D",
            (Self::SplitRight, Platform::MacOs) => "Cmd D",
            (Self::SplitDown, Platform::Linux) => "Ctrl Shift D",
            (Self::SplitDown, Platform::MacOs) => "Cmd Shift D",
            (Self::TerminalMode, Platform::Linux) => "Ctrl Shift 1",
            (Self::TerminalMode, Platform::MacOs) => "Cmd 1",
            (Self::PinnedMode1, Platform::Linux) => "Ctrl Shift 2",
            (Self::PinnedMode1, Platform::MacOs) => "Cmd 2",
            (Self::PinnedMode2, Platform::Linux) => "Ctrl Shift 3",
            (Self::PinnedMode2, Platform::MacOs) => "Cmd 3",
            (Self::PinnedMode3, Platform::Linux) => "Ctrl Shift 4",
            (Self::PinnedMode3, Platform::MacOs) => "Cmd 4",
            (Self::PinnedMode4, Platform::Linux) => "Ctrl Shift 5",
            (Self::PinnedMode4, Platform::MacOs) => "Cmd 5",
            (Self::AllModesMenu, Platform::Linux) => "Ctrl Shift M",
            (Self::AllModesMenu, Platform::MacOs) => "Cmd Shift M",
        }
    }
}

/// The platform a shortcut binding is written for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    /// Linux and every other system that is not macOS: Ctrl Shift
    /// shortcuts.
    Linux,
    /// macOS: Cmd shortcuts.
    MacOs,
}

impl Platform {
    /// Return the platform this build runs on.
    pub fn current() -> Self {
        if cfg!(target_os = "macos") {
            Platform::MacOs
        } else {
            Platform::Linux
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linux_labels_follow_the_spec_table() {
        let labels: Vec<(AppShortcut, &str)> = AppShortcut::ALL
            .into_iter()
            .map(|shortcut| (shortcut, shortcut.label_on(Platform::Linux)))
            .collect();

        assert_eq!(
            labels,
            vec![
                (AppShortcut::CommandPalette, "Ctrl Shift K"),
                (AppShortcut::LeftPanel, "Ctrl Shift B"),
                (AppShortcut::FilesPanel, "Ctrl Shift O"),
                (AppShortcut::FocusMode, "Ctrl Shift F"),
                (AppShortcut::Settings, "Ctrl ,"),
                (AppShortcut::NewTerminal, "Ctrl Shift T"),
                (AppShortcut::CloseSession, "Ctrl Shift W"),
                (AppShortcut::SplitRight, "Super Shift D"),
                (AppShortcut::SplitDown, "Ctrl Shift D"),
                (AppShortcut::TerminalMode, "Ctrl Shift 1"),
                (AppShortcut::PinnedMode1, "Ctrl Shift 2"),
                (AppShortcut::PinnedMode2, "Ctrl Shift 3"),
                (AppShortcut::PinnedMode3, "Ctrl Shift 4"),
                (AppShortcut::PinnedMode4, "Ctrl Shift 5"),
                (AppShortcut::AllModesMenu, "Ctrl Shift M"),
            ]
        );
    }

    #[test]
    fn macos_labels_follow_the_spec_table() {
        let labels: Vec<(AppShortcut, &str)> = AppShortcut::ALL
            .into_iter()
            .map(|shortcut| (shortcut, shortcut.label_on(Platform::MacOs)))
            .collect();

        assert_eq!(
            labels,
            vec![
                (AppShortcut::CommandPalette, "Cmd K"),
                (AppShortcut::LeftPanel, "Cmd B"),
                (AppShortcut::FilesPanel, "Cmd Shift E"),
                (AppShortcut::FocusMode, "Cmd Shift F"),
                (AppShortcut::Settings, "Cmd ,"),
                (AppShortcut::NewTerminal, "Cmd T"),
                (AppShortcut::CloseSession, "Cmd W"),
                (AppShortcut::SplitRight, "Cmd D"),
                (AppShortcut::SplitDown, "Cmd Shift D"),
                (AppShortcut::TerminalMode, "Cmd 1"),
                (AppShortcut::PinnedMode1, "Cmd 2"),
                (AppShortcut::PinnedMode2, "Cmd 3"),
                (AppShortcut::PinnedMode3, "Cmd 4"),
                (AppShortcut::PinnedMode4, "Cmd 5"),
                (AppShortcut::AllModesMenu, "Cmd Shift M"),
            ]
        );
    }

    #[test]
    fn no_two_shortcuts_share_a_label_on_the_same_platform() {
        for platform in [Platform::Linux, Platform::MacOs] {
            for (index, shortcut) in AppShortcut::ALL.iter().enumerate() {
                for other in &AppShortcut::ALL[index + 1..] {
                    assert_ne!(
                        shortcut.label_on(platform),
                        other.label_on(platform),
                        "{shortcut:?} and {other:?} share keys on {platform:?}"
                    );
                }
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

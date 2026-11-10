//! The Settings dialog of the OTTY app: a wide dialog over every mode where
//! the user edits the interface language, the terminal shell and editor, how
//! panes split and the theme, in a draft, and saves them.
//!
//! The feature reads and writes no files. Opening the dialog reports
//! [`SettingsOutcome::ReadSaved`] and Save reports
//! [`SettingsOutcome::WriteSaved`]; `otty-app-settings-io` does the I/O
//! and the answer comes back as a [`SettingsIntent`]. Settings that were saved or read
//! again come out as [`SettingsOutcome::Applied`], and the binary gives
//! their language, theme and shell to the window and new terminals. Parsing
//! the file's text ([`SettingsLoad::parse`]) is plain data work, so it lives
//! here, next to the data.
//!
//! # Example
//!
//! ```
//! use otty_app_settings::{
//!     Settings, SettingsData, SettingsIntent, SettingsOutcome,
//! };
//! use otty_app_slots::Feature;
//!
//! let mut settings = Settings::new(
//!     SettingsData::defaults("/bin/sh"),
//!     String::from("/bin/sh"),
//! );
//!
//! let opened = settings.reduce(SettingsIntent::Open, ());
//! assert_eq!(
//!     opened,
//!     vec![SettingsOutcome::Opened, SettingsOutcome::ReadSaved]
//! );
//!
//! let _ = settings.reduce(SettingsIntent::EditorChanged("vim".into()), ());
//! let save = settings.reduce(SettingsIntent::Save, ());
//! assert!(matches!(
//!     save.as_slice(),
//!     [SettingsOutcome::WriteSaved(draft)] if draft.terminal_editor() == "vim"
//! ));
//! // Only a finished save changes the saved settings.
//! assert_eq!(settings.terminal_editor(), "nano");
//! ```

mod event;
mod model;
mod problem;
mod reducer;
mod state;
mod types;
mod view;

pub use event::{SettingsIntent, SettingsOutcome};
use model::SettingsViewModel;
use otty_app_slots::{Feature, Overlay, Overlays};
use otty_app_theme::theme::ThemeProps;
pub use problem::SettingsProblem;
use state::SettingsState;
pub use types::{
    LanguageSetting, SettingsData, SettingsLoad, SettingsLoadStatus,
};
use view::settings_dialog;

/// The Settings feature: the saved settings the app runs with, and the
/// dialog with its draft while it is open.
///
/// # Example
///
/// ```
/// use otty_app_settings::{Settings, SettingsData};
///
/// let settings = Settings::new(
///     SettingsData::defaults("/bin/sh"),
///     String::from("/bin/sh"),
/// );
///
/// assert!(!settings.is_open());
/// assert_eq!(settings.saved(), &SettingsData::defaults("/bin/sh"));
/// ```
pub struct Settings {
    state: SettingsState,
}

impl Settings {
    /// Return whether the Settings dialog is open.
    pub fn is_open(&self) -> bool {
        self.state.is_open()
    }

    /// Return the saved settings the app runs with. Edits in the open
    /// dialog are not in them until Save.
    pub fn saved(&self) -> &SettingsData {
        self.state.baseline()
    }

    /// Return the saved editor command that opens a file pressed in the
    /// Files panel. An edit in the dialog counts only once it is saved.
    pub fn terminal_editor(&self) -> &str {
        self.state.baseline().terminal_editor()
    }

    /// Return whether the open dialog holds edits that were not saved.
    pub fn has_unsaved_edits(&self) -> bool {
        self.state.is_dirty()
    }

    /// Create the feature with the settings read at start, the dialog
    /// closed. `default_shell` is the system shell the binary read at
    /// start: a shell emptied in the dialog is saved as it.
    pub fn new(saved: SettingsData, default_shell: String) -> Self {
        Self {
            state: SettingsState::from_settings(saved, default_shell),
        }
    }
}

impl Feature for Settings {
    type Intent = SettingsIntent;
    type Outcome = SettingsOutcome;
    type Ctx<'a> = ();

    fn reduce(
        &mut self,
        intent: SettingsIntent,
        _ctx: (),
    ) -> Vec<SettingsOutcome> {
        reducer::reduce(&mut self.state, intent)
    }
}

impl Overlays for Settings {
    /// The open dialog: wide, in the middle of the area the frame gives,
    /// over a dimmed layer. Unlike the other dialogs, a press on the dimmed
    /// layer closes it, like Esc and the close cross.
    fn overlay<'a>(
        &'a self,
        theme: ThemeProps<'a>,
    ) -> Option<Overlay<'a, SettingsIntent>> {
        let vm = SettingsViewModel::from_state(&self.state)?;

        Some(Overlay::dialog(settings_dialog::view(
            settings_dialog::SettingsDialogProps { vm, theme },
        )))
    }
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use otty_app_i18n::{self as i18n, Key};
    use otty_app_slots::OverlayKind;
    use otty_app_theme::theme::ThemeManager;

    use super::*;

    /// Settings is a dialog: the frame ignores shortcuts and presses
    /// outside it, and Esc means Cancel.
    #[test]
    fn the_open_settings_are_a_dialog_overlay() {
        let themes = ThemeManager::new();
        let theme = ThemeProps::new(themes.current());
        let mut settings = Settings::new(
            SettingsData::defaults("/bin/sh"),
            String::from("/bin/sh"),
        );
        let closed = settings.overlay(theme).is_none();

        let _ = settings.reduce(SettingsIntent::Open, ());

        let kind = settings.overlay(theme).map(|open| open.kind());
        assert!(closed);
        assert_eq!(kind, Some(OverlayKind::Dialog));
    }

    /// The dialog draws the draft, not the saved settings.
    #[test]
    fn the_dialog_shows_the_draft() {
        let themes = ThemeManager::new();
        let theme = ThemeProps::new(themes.current());
        let mut settings = Settings::new(
            SettingsData::defaults("/bin/sh"),
            String::from("/bin/sh"),
        );
        let _ = settings.reduce(SettingsIntent::Open, ());
        let _ = settings.reduce(
            SettingsIntent::EditorChanged(String::from("otty-test-editor")),
            (),
        );

        let dialog = settings
            .overlay(theme)
            .expect("the dialog should be open")
            .into_content();
        let mut ui = simulator(dialog);

        assert!(ui.find(i18n::t(Key::SettingsTitle)).is_ok());
        assert!(ui.find("otty-test-editor").is_ok());
    }

    /// Opening and editing changes the draft only; the saved settings and
    /// the editor the Files panel opens files with change on a finished
    /// save.
    #[test]
    fn the_saved_editor_changes_only_when_the_save_finishes() {
        let mut settings = Settings::new(
            SettingsData::defaults("/bin/sh"),
            String::from("/bin/sh"),
        );
        let _ = settings.reduce(SettingsIntent::Open, ());
        let _ = settings
            .reduce(SettingsIntent::EditorChanged(String::from("vim")), ());
        let edited = settings.has_unsaved_edits();
        let before_save = settings.terminal_editor().to_string();

        let written = settings.reduce(SettingsIntent::Save, ());
        let Some(SettingsOutcome::WriteSaved(draft)) = written.first() else {
            panic!("Save should ask to write the draft: {written:?}");
        };
        let _ =
            settings.reduce(SettingsIntent::SaveCompleted(draft.clone()), ());

        assert!(edited);
        assert_eq!(before_save, "nano");
        assert_eq!(settings.terminal_editor(), "vim");
        assert_eq!(settings.saved().terminal_editor(), "vim");
        assert!(!settings.has_unsaved_edits());
    }

    /// A shell emptied in the dialog is saved as the system shell the
    /// binary gave at start.
    #[test]
    fn an_emptied_shell_is_saved_as_the_given_default_shell() {
        let mut settings = Settings::new(
            SettingsData::defaults("/bin/zsh"),
            String::from("/usr/bin/otty-system-shell"),
        );
        let _ = settings.reduce(SettingsIntent::Open, ());
        let _ =
            settings.reduce(SettingsIntent::ShellChanged(String::new()), ());

        let written = settings.reduce(SettingsIntent::Save, ());

        assert!(matches!(
            written.as_slice(),
            [SettingsOutcome::WriteSaved(draft)]
                if draft.terminal_shell() == "/usr/bin/otty-system-shell"
        ));
    }
}

//! Delivers the intents of the Settings dialog and reads what it reports:
//! the file reads and writes, the saved settings the window takes, the
//! keyboard and the failures to tell.

use iced::Task;
use otty_app_notifications::NotificationsIntent;
use otty_app_settings::{SettingsData, SettingsIntent, SettingsOutcome};
use otty_app_slots::Feature;
use otty_app_terminal::TerminalIntent;

use super::{notifications, terminal};
use crate::app::App;
use crate::effects;
use crate::message::AppEvent;

/// Give a settings intent to the feature and read what it reports.
pub(crate) fn deliver(app: &mut App, intent: SettingsIntent) -> Task<AppEvent> {
    let outcomes = app.settings.reduce(intent, ());

    let tasks: Vec<Task<AppEvent>> = outcomes
        .into_iter()
        .map(|outcome| read(app, outcome))
        .collect();

    Task::batch(tasks)
}

fn read(app: &mut App, outcome: SettingsOutcome) -> Task<AppEvent> {
    match outcome {
        // The open dialog takes the keyboard from the terminal, so typed
        // keys do not reach the shell and Esc reaches the frame.
        SettingsOutcome::Opened => super::take_keyboard(),
        SettingsOutcome::ReadSaved => otty_app_settings_io::read(
            &app.config_dir,
            app.default_shell.clone(),
        )
        .map(AppEvent::Settings),
        SettingsOutcome::WriteSaved(settings) => {
            otty_app_settings_io::write(&app.config_dir, settings)
                .map(AppEvent::Settings)
        },
        SettingsOutcome::Applied(settings) => apply(app, &settings),
        // The terminal decides: in Terminal mode the active pane gets its
        // keys back; in Containers mode nothing takes them.
        SettingsOutcome::Closed => {
            terminal::deliver(app, TerminalIntent::FocusActive)
        },
        // Applied settings come first, so the text is in their language.
        SettingsOutcome::Failed(problem) => notifications::deliver(
            app,
            NotificationsIntent::Push(problem.text()),
        ),
    }
}

/// Give saved settings to the window and the terminals: the language of
/// the interface and of the fonts, the theme with its colors for the open
/// terminals, and the shell with its integration files, written in the
/// background.
///
/// New terminals take the theme from `app.theme_manager` and the shell
/// from `app.shell_session`; the saved editor and the equalize-panes switch
/// stay in `app.settings.saved()`.
fn apply(app: &mut App, saved: &SettingsData) -> Task<AppEvent> {
    effects::locale::apply(saved.language().fixed());
    app.theme_manager.set_theme(saved.theme());
    let palette = Box::new(app.theme_manager.current().terminal_palette());
    let recolored =
        terminal::deliver(app, TerminalIntent::ApplyTheme { palette });

    let integrated = effects::shell_integration::integrate(
        saved.terminal_shell().to_string(),
        app.config_dir.clone(),
    );

    Task::batch([recolored, integrated])
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use otty_app_settings::{
        SettingsLoad, SettingsLoadStatus, SettingsProblem,
    };
    use otty_app_theme::theme::BuiltinTheme;

    use super::*;
    use crate::testing::task_outputs::{
        collect_outputs, focuses_nothing, runs_widget_operation,
    };
    use crate::testing::window::{
        app, edit_the_shell, open_settings, open_shell_tab, test_settings,
    };
    use crate::view::view;

    /// Saved settings with the light theme, that apply without switching
    /// the language of the other tests or writing shell files.
    fn light_settings() -> SettingsData {
        let mut saved = test_settings();
        saved.set_theme(BuiltinTheme::OttyLight);

        saved
    }

    /// The open dialog takes the keyboard from the terminal, as any
    /// overlay does, and the saved settings are read again.
    #[test]
    fn an_opened_dialog_takes_the_keyboard_and_reads_the_file() {
        let mut app = app();

        let task = deliver(&mut app, SettingsIntent::Open);

        assert!(app.settings.is_open());
        assert!(focuses_nothing(task));
    }

    /// Save keeps the dialog open; the written settings come back as
    /// SaveCompleted and the window takes their theme.
    #[test]
    fn a_finished_save_gives_the_window_the_saved_theme_and_keeps_the_dialog() {
        let mut app = app();
        open_settings(&mut app);

        let task =
            deliver(&mut app, SettingsIntent::SaveCompleted(light_settings()));

        assert!(app.settings.is_open());
        assert_eq!(app.theme().to_string(), BuiltinTheme::OttyLight.title());
        assert_eq!(app.settings.saved(), &light_settings());
        // The shell of the saved settings is integrated in the background
        // and becomes the session new terminals start.
        let events = collect_outputs(task);
        assert!(matches!(
            events.as_slice(),
            [AppEvent::ShellIntegrated(session)]
                if session.name() == "sh"
        ));
    }

    /// The settings read when the dialog opens apply too: someone may have
    /// changed the file since the start.
    #[test]
    fn settings_read_again_give_the_window_their_theme() {
        let mut app = app();
        open_settings(&mut app);

        let _shell = deliver(
            &mut app,
            SettingsIntent::ReloadLoaded(SettingsLoad::new(
                light_settings(),
                SettingsLoadStatus::Loaded,
            )),
        );

        assert_eq!(app.theme().to_string(), BuiltinTheme::OttyLight.title());
    }

    /// A failed read, a failed save and a broken file each become a toast.
    #[test]
    fn a_failure_becomes_a_toast() {
        let failures = [
            (
                SettingsIntent::ReloadFailed(String::from("no access")),
                SettingsProblem::ReadFailed(String::from("no access")),
            ),
            (
                SettingsIntent::SaveFailed(String::from("disk full")),
                SettingsProblem::SaveFailed(String::from("disk full")),
            ),
        ];

        for (failure, problem) in failures {
            let mut app = app();
            open_settings(&mut app);

            let _none = deliver(&mut app, failure);

            let mut ui = simulator(view(&app));
            assert_eq!(app.notifications.unseen_count(), 1);
            assert!(ui.find(problem.text()).is_ok(), "{problem:?}");
        }
    }

    #[test]
    fn a_broken_file_applies_the_defaults_and_becomes_a_toast() {
        let mut app = app();
        app.theme_manager.set_theme(BuiltinTheme::OttyLight);
        open_settings(&mut app);
        edit_the_shell(&mut app, "/bin/edited");

        let _shell = deliver(
            &mut app,
            SettingsIntent::ReloadLoaded(SettingsLoad::new(
                test_settings(),
                SettingsLoadStatus::Invalid(String::from("expected value")),
            )),
        );

        let problem =
            SettingsProblem::FileInvalid(String::from("expected value"));
        assert_eq!(app.theme().to_string(), BuiltinTheme::OttyDark.title());
        assert!(!app.settings.has_unsaved_edits());
        assert_eq!(app.notifications.unseen_count(), 1);
        assert!(simulator(view(&app)).find(problem.text()).is_ok());
    }

    /// Closing drops the edits; without a terminal tab nothing takes the
    /// keyboard after it.
    #[test]
    fn a_closed_dialog_drops_the_edits() {
        let mut app = app();
        open_settings(&mut app);
        edit_the_shell(&mut app, "/bin/edited");

        let task = deliver(&mut app, SettingsIntent::Close);

        assert!(!app.settings.is_open());
        assert!(!app.settings.has_unsaved_edits());
        assert_eq!(task.units(), 0);
    }

    /// Owner, Q1 variant A of the old app: in Terminal mode the active pane
    /// gets its keys back when Settings closes.
    #[test]
    fn closing_settings_gives_the_pane_its_keys_in_terminal_mode() {
        let mut app = app();
        let _tab = open_shell_tab(&mut app);
        open_settings(&mut app);

        let task = deliver(&mut app, SettingsIntent::Close);

        assert!(!app.settings.is_open());
        assert!(runs_widget_operation(task));
    }

    /// Saved settings bring their theme colors to the terminals: new ones
    /// start with them (the open ones take them too, see the terminal
    /// feature).
    #[test]
    fn saved_settings_color_new_terminals() {
        let mut app = app();
        let _tab = open_shell_tab(&mut app);
        let before = render_background(&app);

        let _shell =
            deliver(&mut app, SettingsIntent::SaveCompleted(light_settings()));

        assert_ne!(render_background(&app), before);
        assert_eq!(
            render_background(&app),
            BuiltinTheme::OttyLight.terminal_palette().background
        );
    }

    /// The background new terminals start with now.
    fn render_background(app: &App) -> String {
        crate::dispatch::terminal::base_settings(app)
            .theme
            .color_pallete
            .background
            .clone()
    }

    /// Save writes in the background; the saved settings stay as they were
    /// until the answer comes back.
    #[test]
    fn save_writes_in_the_background() {
        let mut app = app();
        open_settings(&mut app);
        edit_the_shell(&mut app, "/bin/edited");

        let task = deliver(&mut app, SettingsIntent::Save);

        assert_eq!(task.units(), 1);
        assert_eq!(
            app.settings.saved().terminal_shell(),
            test_settings().terminal_shell()
        );
    }
}

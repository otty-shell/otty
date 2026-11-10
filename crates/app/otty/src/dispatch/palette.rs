//! Delivers the intents of the command palette. What it lists and which
//! commands can run now are gathered here, from the other features, into
//! its `Ctx` before every intent.

use iced::Task;
use iced::widget::operation;
use otty_app_palette::{PaletteCtx, PaletteIntent, PaletteOutcome};
use otty_app_slots::Feature;
use otty_ui_kit::components::command_palette::input_id;

use super::command;
use crate::app::App;
use crate::guards::command_ctx;
use crate::message::AppEvent;

/// Give a palette intent to the feature, with what it lists, and read what
/// it reports.
pub(crate) fn deliver(app: &mut App, intent: PaletteIntent) -> Task<AppEvent> {
    let ctx = palette_ctx(app);

    let outcomes = app.palette.reduce(intent, ctx);

    let tasks: Vec<Task<AppEvent>> = outcomes
        .into_iter()
        .map(|outcome| read(app, outcome))
        .collect();

    Task::batch(tasks)
}

/// Gather what the palette lists: the pinned modes, for the mode shortcut
/// labels, and the commands that can run now. The palette lists and runs
/// only those (see `AppCommand::is_available`).
///
/// The tabs feature (issue 10) adds the open tabs, grouped by machine as in
/// the Sessions panel, and the quick launch feature (issue 12) every launch
/// in tree order. They borrow from those features, so this is then built
/// from the fields of `App`, apart from the palette it is given to.
fn palette_ctx(app: &App) -> PaletteCtx<'static> {
    let pinned_modes = app
        .modes
        .vm()
        .pinned_rows
        .iter()
        .map(|row| row.mode)
        .collect();

    PaletteCtx {
        sessions: Vec::new(),
        launches: Vec::new(),
        pinned_modes,
        commands: command_ctx(app),
    }
}

fn read(app: &mut App, outcome: PaletteOutcome) -> Task<AppEvent> {
    match outcome {
        // Moving the focus to the input also takes it from the terminal
        // panes, so typed keys reach the palette.
        PaletteOutcome::FocusInput => operation::focus(input_id()),
        // The tabs feature (issue 10) switches to the tab, as a press on
        // its row in the Sessions panel does.
        PaletteOutcome::SessionPicked { tab_id: _ } => Task::none(),
        // The quick launch feature (issue 12) runs the launch, as the New
        // session menu does.
        PaletteOutcome::LaunchPicked { path: _ } => Task::none(),
        PaletteOutcome::CommandPicked(picked) => command::deliver(app, picked),
        // In Terminal mode the terminal feature (issue 11) gives the active
        // pane its keys back; in Containers mode nothing takes them.
        PaletteOutcome::KeyboardReleased => Task::none(),
    }
}

#[cfg(test)]
mod tests {
    use otty_app_domain::commands::CommandCtx;
    use otty_app_domain::mode::Mode;
    use otty_app_i18n::{self as i18n, Key};
    use otty_ui_kit::components::command_palette::PaletteAction;

    use super::*;
    use crate::modes::ModesIntent;
    use crate::sidebar::SidebarIntent;
    use crate::testing::task_outputs::focuses_widget;
    use crate::testing::window::{app, open_palette};

    /// Type `query` in the open palette and pick the entry on top.
    fn pick(app: &mut App, query: &str) -> Task<AppEvent> {
        let _ = deliver(
            app,
            PaletteIntent::Action(PaletteAction::QueryChanged(String::from(
                query,
            ))),
        );

        deliver(app, PaletteIntent::Action(PaletteAction::RunActive))
    }

    #[test]
    fn the_palette_lists_terminal_and_the_pinned_modes_and_reads_the_mode() {
        let mut app = app();
        let in_terminal = palette_ctx(&app);
        let _ = app.modes.reduce(ModesIntent::Unpin(Mode::Containers), ());
        let _ = app.modes.reduce(ModesIntent::Select(Mode::Containers), ());

        let in_containers = palette_ctx(&app);

        assert_eq!(
            in_terminal.pinned_modes,
            [Mode::Terminal, Mode::Containers]
        );
        assert_eq!(
            in_terminal.commands,
            CommandCtx {
                is_terminal_mode: true,
                has_active_tab: false,
                has_focused_pane: false,
            }
        );
        assert_eq!(in_containers.pinned_modes, [Mode::Terminal]);
        assert!(!in_containers.commands.is_terminal_mode);
    }

    #[test]
    fn an_opened_palette_gives_its_input_the_keyboard() {
        let mut app = app();

        let task = deliver(&mut app, PaletteIntent::Open);

        assert!(app.palette.is_open());
        assert!(focuses_widget(task, &input_id()));
    }

    /// A picked command runs in the same update, through the same path as
    /// its shortcut.
    #[test]
    fn a_picked_command_runs_in_the_same_update() {
        let mut app = app();
        open_palette(&mut app);

        let _ = pick(&mut app, i18n::t(Key::WindowFocusMode));

        assert!(!app.palette.is_open());
        assert!(app.sidebar.vm().is_focus_mode);
    }

    /// The palette entry of the bell opens the notifications list, which
    /// takes the keyboard as any menu does.
    #[test]
    fn the_notifications_entry_opens_the_list() {
        let mut app = app();
        open_palette(&mut app);

        let _ = pick(&mut app, i18n::t(Key::NotificationsTitle));

        assert!(!app.palette.is_open());
        assert!(app.notifications.is_list_open());
    }

    /// Every mode can be switched to from the palette, pinned or not.
    #[test]
    fn a_picked_mode_switches_to_it() {
        let mut app = app();
        let _ = app.modes.reduce(ModesIntent::Unpin(Mode::Containers), ());
        open_palette(&mut app);

        let _ = pick(&mut app, i18n::t(Key::PaletteSwitchToContainersMode));

        assert_eq!(app.modes.active(), Mode::Containers);
    }

    /// New folder acts on the Launches panel, which shows only in Terminal
    /// mode: the palette does not list it in Containers mode.
    #[test]
    fn a_command_that_can_not_run_now_is_not_listed() {
        let mut app = app();
        let _ = app.modes.reduce(ModesIntent::Select(Mode::Containers), ());
        let _ = app.sidebar.reduce(SidebarIntent::ToggleLeft, ());
        open_palette(&mut app);

        let _ = pick(&mut app, i18n::t(Key::PaletteNewFolder));

        assert!(app.palette.is_open());
        assert!(!app.sidebar.vm().is_left_open);
    }

    /// Until the terminal feature moves in, nothing takes the keyboard
    /// after the palette closes.
    #[test]
    fn a_closed_palette_moves_no_focus_yet() {
        let mut app = app();
        open_palette(&mut app);

        let task = deliver(&mut app, PaletteIntent::Dismiss);

        assert!(!app.palette.is_open());
        assert_eq!(task.units(), 0);
    }
}

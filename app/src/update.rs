use iced::Task;

use super::{App, AppEvent};
use crate::commands::AppCommand;
use crate::guards::{
    MenuGuard, Overlay, context_menu_guard, dialog_guard, inline_edit_guard,
    is_escape_press, is_header_menu_choice, is_terminal_input_kept_out,
    top_overlay,
};
use crate::router;
use crate::shortcuts::{AppShortcut, Platform, shortcut_for_key_press};
use crate::widgets::containers::actions::event::ActionsIntent;
use crate::widgets::containers::event::{ContainersEvent, ContainersIntent};
use crate::widgets::modes::event::{ModesEvent, ModesIntent};
use crate::widgets::palette::event::{PaletteEvent, PaletteIntent};
use crate::widgets::quick_launch::event::{
    QuickLaunchEvent, QuickLaunchIntent,
};
use crate::widgets::sidebar::event::{SidebarEvent, SidebarIntent};
use crate::widgets::terminal_workspace::event::{
    TerminalWorkspaceEvent, TerminalWorkspaceIntent,
};

/// Handle input from the user. Open menus, the command palette and an inline
/// edit decide first what happens to it, once, on what the user did.
pub(super) fn update_input(app: &mut App, event: AppEvent) -> Task<AppEvent> {
    let event = as_command(event);
    let task = filter_input(app, event);

    finish_update(app, task)
}

/// Handle an event that does not come from the user directly: it skips the
/// checks of [`update_input`], so it can not get lost.
pub(super) fn update_internal(
    app: &mut App,
    event: AppEvent,
) -> Task<AppEvent> {
    let task = router::route(app, event);

    finish_update(app, task)
}

/// Tell the Containers widget when the event changed whether the right panel
/// is on screen. The sidebar alone knows that, and many paths change it, so it
/// is read here, after every event, instead of by each path.
fn finish_update(app: &mut App, task: Task<AppEvent>) -> Task<AppEvent> {
    let right_panel = tell_containers_about_right_panel(app);

    Task::batch([task, right_panel])
}

fn filter_input(app: &mut App, event: AppEvent) -> Task<AppEvent> {
    let top = top_overlay(app);

    // Keys typed under a menu or a dialog belong to it, even if a terminal
    // still had keyboard focus.
    if is_terminal_input_kept_out(top, &event) {
        return Task::none();
    }

    // Esc closes the overlay on top, or cancels it for a dialog and the
    // inline edit. A focused terminal keeps Esc for its shell, so every
    // overlay takes the focus from the terminals when it opens.
    if let Some(top) = top
        && is_escape_press(&event)
    {
        return router::route(app, top.escape_event());
    }

    // A dialog waits for its answer. It lies over any menu, and the menu
    // stays open under it until the dialog is answered.
    if let Some(dialog) = top.filter(|top| top.is_dialog()) {
        return match dialog_guard(dialog, &event) {
            MenuGuard::Allow => router::route(app, event),
            _ => Task::none(),
        };
    }

    // Mode shortcuts still work while the All modes menu is open: the menu
    // shortcut closes it and a mode shortcut switches, which closes it too.
    // Every other key stays ignored like for the other menus.
    if app.widgets.modes.is_menu_open() && is_mode_shortcut(&event) {
        return router::route(app, event);
    }

    // The palette shortcut under the open palette gives its input the focus
    // again, and the palette keeps what was typed (owner, Q1).
    if top == Some(Overlay::Palette) && is_palette_shortcut(&event) {
        return router::route(app, event);
    }

    let mut pre_dispatch_tasks = Vec::new();

    if app.widgets.quick_launch.has_inline_edit() && inline_edit_guard(&event) {
        pre_dispatch_tasks.push(router::route(
            app,
            AppEvent::QuickLaunch(QuickLaunchEvent::Intent(
                QuickLaunchIntent::CancelInlineEdit,
            )),
        ));
    }

    if app.widgets.sidebar.has_header_menu_open()
        && is_header_menu_choice(&event)
    {
        pre_dispatch_tasks.push(router::route(
            app,
            AppEvent::Sidebar(SidebarEvent::Intent(
                SidebarIntent::HeaderMenuDismiss,
            )),
        ));
    } else if top.is_some_and(Overlay::is_menu) {
        match context_menu_guard(&event) {
            MenuGuard::Allow => {},
            MenuGuard::Ignore => return Task::none(),
            MenuGuard::Dismiss => return close_all_menus(app),
            MenuGuard::DismissAndRun => {
                pre_dispatch_tasks.push(close_all_menus(app));
            },
        }
    }

    let dispatch_task = router::route(app, event);
    if pre_dispatch_tasks.is_empty() {
        dispatch_task
    } else {
        pre_dispatch_tasks.push(dispatch_task);
        Task::batch(pre_dispatch_tasks)
    }
}

/// Turn a key press that is an app shortcut into its command, on both
/// routes a key press can take (see [`crate::shortcuts`]): a key no widget
/// captured, and a binding a focused terminal reported. The checks then see
/// a shortcut once, as a command, whichever route it took. Every other
/// event stays as it is.
fn as_command(event: AppEvent) -> AppEvent {
    let command = match &event {
        AppEvent::Keyboard(key) => {
            shortcut_for_key_press(key, Platform::current())
                .map(AppShortcut::command)
        },
        AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
            TerminalWorkspaceIntent::Widget(
                otty_ui_term::Event::BindingActionDispatched { action, .. },
            ),
        )) => Some(action.command()),
        _ => None,
    };

    match command {
        Some(command) => AppEvent::Command(command),
        None => event,
    }
}

/// Send the Containers widget the state of the right panel, if it changed.
fn tell_containers_about_right_panel(app: &mut App) -> Task<AppEvent> {
    let is_hidden = !app.widgets.sidebar.vm().is_right_open;
    if app.widgets.containers.is_right_panel_hidden() == is_hidden {
        return Task::none();
    }

    router::route(
        app,
        AppEvent::Containers(ContainersEvent::Intent(
            ContainersIntent::RightPanelHidden(is_hidden),
        )),
    )
}

fn is_mode_shortcut(event: &AppEvent) -> bool {
    matches!(
        event,
        AppEvent::Command(
            AppCommand::SelectModeAt { .. } | AppCommand::ToggleModesMenu
        )
    )
}

fn is_palette_shortcut(event: &AppEvent) -> bool {
    matches!(event, AppEvent::Command(AppCommand::OpenPalette))
}

/// Close every menu: the one on top and any left open under it.
fn close_all_menus(app: &mut App) -> Task<AppEvent> {
    Task::batch(vec![
        router::route(
            app,
            AppEvent::Palette(PaletteEvent::Intent(PaletteIntent::Dismiss)),
        ),
        router::route(
            app,
            AppEvent::Sidebar(SidebarEvent::Intent(
                SidebarIntent::HeaderMenuDismiss,
            )),
        ),
        router::route(
            app,
            AppEvent::QuickLaunch(QuickLaunchEvent::Intent(
                QuickLaunchIntent::ContextMenuDismiss,
            )),
        ),
        router::route(
            app,
            AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
                TerminalWorkspaceIntent::CloseAllContextMenus,
            )),
        ),
        router::route(
            app,
            AppEvent::Containers(ContainersEvent::Intent(
                ContainersIntent::Actions(ActionsIntent::CloseRowMenu),
            )),
        ),
        close_modes_menu(app),
    ])
}

fn close_modes_menu(app: &mut App) -> Task<AppEvent> {
    router::route(
        app,
        AppEvent::Modes(ModesEvent::Intent(ModesIntent::CloseMenu)),
    )
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use iced::keyboard::key::{Code, Named, Physical};
    use iced::keyboard::{Event, Key, Location, Modifiers};
    use iced::widget::pane_grid;
    use otty_containers::{EngineKind, fixtures};
    use otty_ui_kit::components::command_palette::PaletteAction;
    use otty_ui_term::settings::Settings;

    use super::*;
    use crate::app::Message;
    use crate::domain::exit_behavior::ExitBehavior;
    use crate::domain::mode::Mode;
    use crate::guards::Overlay;
    use crate::i18n;
    use crate::testing::keyboard;
    use crate::testing::overlays::{
        open_confirm, open_create_dialog, open_header_menu, open_launch_menu,
        open_modes_menu, open_palette, open_pane_menu, open_row_menu,
        open_secret_prompt, open_settings,
    };
    use crate::testing::shell_tab::open_shell_tab;
    use crate::testing::task_outputs::{
        collect_outputs, runs_widget_operation,
    };
    use crate::testing::window::{
        app_in, containers_app_with_lists, open_launch_editor_intent,
        show_toast,
    };
    use crate::widgets::chrome::event::{ChromeEvent, ChromeIntent};
    use crate::widgets::containers::actions::types::RowAction;
    use crate::widgets::containers::event::ContainersEffect;
    use crate::widgets::containers::inspector::event::InspectorIntent;
    use crate::widgets::containers::inspector::types::LogTarget;
    use crate::widgets::containers::run::event::{RunEffect, RunIntent};
    use crate::widgets::containers::run::types::RunEdit;
    use crate::widgets::containers::types::{InspectorTab, LoadedLists};
    use crate::widgets::explorer::event::{ExplorerEvent, ExplorerIntent};
    use crate::widgets::modes::ModesWidget;
    use crate::widgets::settings::event::{SettingsEvent, SettingsIntent};
    use crate::widgets::sidebar::types::HeaderMenu;
    use crate::widgets::tabs::event::{TabsEvent, TabsIntent};
    use crate::widgets::tabs::types::TabMachine;

    const FOLDER: &str = "/srv/site";

    fn app_with_open_modes_menu() -> App {
        let (mut app, _) = App::new();
        app.widgets.modes = ModesWidget::new();
        let _ = update_input(
            &mut app,
            AppEvent::Modes(ModesEvent::Intent(ModesIntent::ToggleMenu)),
        );
        app
    }

    fn escape_press() -> AppEvent {
        AppEvent::Keyboard(Event::KeyPressed {
            key: Key::Named(Named::Escape),
            modified_key: Key::Named(Named::Escape),
            physical_key: Physical::Code(Code::Escape),
            location: Location::Standard,
            modifiers: Modifiers::empty(),
            text: None,
            repeat: false,
        })
    }

    /// The key press the keyboard subscription delivers for `shortcut`.
    fn shortcut_press(shortcut: AppShortcut) -> AppEvent {
        AppEvent::Keyboard(keyboard::shortcut_press(shortcut))
    }

    /// Press `shortcut` and run the events its tasks produce through
    /// `update` again, as the iced runtime does.
    fn press(app: &mut App, shortcut: AppShortcut) {
        let follow_ups =
            collect_outputs(update_input(app, shortcut_press(shortcut)));

        for event in follow_ups {
            let _ = update_internal(app, event);
        }
    }

    #[test]
    fn all_modes_shortcut_closes_the_open_modes_menu() {
        let mut app = app_with_open_modes_menu();

        press(&mut app, AppShortcut::AllModesMenu);

        assert!(!app.widgets.modes.is_menu_open());
    }

    #[test]
    fn mode_shortcut_switches_while_the_modes_menu_is_open() {
        let mut app = app_with_open_modes_menu();

        press(&mut app, AppShortcut::PinnedMode1);

        assert_eq!(app.widgets.modes.active(), Mode::Containers);
        assert!(!app.widgets.modes.is_menu_open());
    }

    #[test]
    fn other_shortcuts_close_the_modes_menu_and_run() {
        let mut app = app_with_open_modes_menu();

        press(&mut app, AppShortcut::FocusMode);

        assert!(!app.widgets.modes.is_menu_open());
        assert!(app.widgets.sidebar.vm().is_focus_mode);
    }

    #[test]
    fn esc_closes_the_open_modes_menu() {
        let mut app = app_with_open_modes_menu();
        assert!(app.widgets.modes.is_menu_open());

        let _ = update_input(&mut app, escape_press());

        assert!(!app.widgets.modes.is_menu_open());
    }

    /// Opens one overlay on an app.
    type Opener = fn(&mut App);

    /// The openers of the menus of Terminal mode and of both modes.
    const TERMINAL_MENUS: [Opener; 5] = [
        open_palette,
        open_modes_menu,
        open_header_menu,
        open_pane_menu,
        open_launch_menu,
    ];

    /// Scenarios 1, 3, 4 and 6: Esc closes the menu on top, wherever the
    /// keyboard focus is.
    #[test]
    fn esc_closes_each_menu() {
        for open in TERMINAL_MENUS {
            let mut app = app_in(Mode::Terminal);
            open(&mut app);
            let opened = top_overlay(&app);

            let _ = update_input(&mut app, escape_press());

            assert!(opened.is_some());
            assert_eq!(top_overlay(&app), None, "{opened:?}");
        }
        let mut app = containers_app_with_lists();
        open_row_menu(&mut app);

        let _ = update_input(&mut app, escape_press());

        assert_eq!(top_overlay(&app), None);
    }

    /// A toast is no overlay: Esc closes the menu on top and leaves the
    /// toast on screen.
    #[test]
    fn esc_closes_a_menu_and_leaves_the_toasts() {
        let mut app = app_in(Mode::Terminal);
        show_toast(&mut app, "disk full");
        open_modes_menu(&mut app);

        let _ = update_input(&mut app, escape_press());
        let mut ui = iced_test::simulator(crate::app::view::view(&app));

        assert!(!app.widgets.modes.is_menu_open());
        assert!(ui.find("disk full").is_ok());
    }

    /// Scenarios 5 and 6: Esc means Cancel in a dialog and in the inline edit.
    #[test]
    fn esc_cancels_each_dialog_and_the_inline_edit() {
        let mut prompt = app_in(Mode::Terminal);
        open_secret_prompt(&mut prompt);
        let mut edit = app_in(Mode::Terminal);
        start_inline_edit(&mut edit);
        let mut create = containers_app_with_lists();
        open_create_dialog(&mut create);
        let mut confirm = containers_app_with_lists();
        open_confirm(&mut confirm);

        for mut app in [prompt, edit, create, confirm] {
            let opened = top_overlay(&app);

            let _ = update_input(&mut app, escape_press());

            assert!(opened.is_some());
            assert_eq!(top_overlay(&app), None, "{opened:?}");
        }
    }

    /// The SSH prompt over a menu over the inline edit: each Esc closes the
    /// one on top and leaves the others open.
    #[test]
    fn esc_closes_only_the_overlay_on_top() {
        let mut app = app_in(Mode::Terminal);
        start_inline_edit(&mut app);
        open_header_menu(&mut app);
        open_secret_prompt(&mut app);
        let mut on_top = Vec::new();

        for _ in 0..3 {
            let _ = update_input(&mut app, escape_press());
            on_top.push(top_overlay(&app));
        }

        assert_eq!(
            on_top,
            [Some(Overlay::HeaderMenu), Some(Overlay::InlineEdit), None]
        );
    }

    #[test]
    fn esc_with_nothing_open_changes_nothing() {
        let mut app = app_in(Mode::Terminal);

        let events = collect_outputs(update_input(&mut app, escape_press()));

        assert!(events.is_empty());
        assert_eq!(top_overlay(&app), None);
    }

    #[test]
    fn pin_and_unpin_keep_the_modes_menu_open() {
        let mut app = app_with_open_modes_menu();

        let _ = update_input(
            &mut app,
            AppEvent::Modes(ModesEvent::Intent(ModesIntent::Unpin(
                Mode::Containers,
            ))),
        );

        assert!(app.widgets.modes.is_menu_open());
        assert_eq!(app.widgets.modes.vm().pinned_count, 0);
    }

    #[test]
    fn choosing_a_mode_in_the_menu_switches_and_closes_it() {
        let mut app = app_with_open_modes_menu();

        let _ = update_input(
            &mut app,
            AppEvent::Modes(ModesEvent::Intent(ModesIntent::Select(
                Mode::Containers,
            ))),
        );

        assert_eq!(app.widgets.modes.active(), Mode::Containers);
        assert!(!app.widgets.modes.is_menu_open());
    }

    #[test]
    fn another_app_action_closes_the_modes_menu_like_the_header_menus() {
        let mut app = app_with_open_modes_menu();
        let was_left_open = app.widgets.sidebar.vm().is_left_open;

        let _ = update_input(
            &mut app,
            AppEvent::Sidebar(SidebarEvent::Intent(SidebarIntent::ToggleLeft)),
        );

        assert!(!app.widgets.modes.is_menu_open());
        assert_eq!(app.widgets.sidebar.vm().is_left_open, was_left_open);
    }

    /// The All modes menu is open and the Run form has a filled row from the
    /// folder [`FOLDER`], with no question out yet.
    fn app_with_modes_menu_over_a_folder_row() -> App {
        let mut app = app_with_open_modes_menu();
        let containers = &mut app.widgets.containers;

        let _ = containers.reduce(ContainersIntent::Enter);
        let _ = containers.reduce(ContainersIntent::DiscoveryFinished(
            fixtures::discovery(&[EngineKind::Docker]),
        ));
        let _ = containers.reduce(ContainersIntent::OpenCreate);
        let _ = containers.reduce(ContainersIntent::Run(RunIntent::Edited(
            RunEdit::AddVolume,
        )));
        let _ = containers.reduce(ContainersIntent::Run(RunIntent::Edited(
            RunEdit::VolumeSource(0, FOLDER.to_string()),
        )));
        let _ = containers.reduce(ContainersIntent::Run(RunIntent::Edited(
            RunEdit::VolumeTarget(0, String::from("/data")),
        )));

        app
    }

    fn containers_intent(intent: ContainersIntent) -> AppEvent {
        AppEvent::Containers(ContainersEvent::Intent(intent))
    }

    /// The message under the first volume row of the Run form.
    fn row_problem(app: &App) -> Option<&'static str> {
        let form = app.widgets.containers.run_form_vm().expect("form");

        form.volumes[0].problem
    }

    #[test]
    fn the_folder_timer_leaves_the_modes_menu_open() {
        let mut app = app_with_modes_menu_over_a_folder_row();

        let _ = update_internal(
            &mut app,
            containers_intent(ContainersIntent::Run(RunIntent::RecheckFolders)),
        );

        assert!(app.widgets.modes.is_menu_open());
    }

    #[test]
    fn a_folder_answer_under_an_open_menu_reaches_the_form() {
        let mut app = app_with_modes_menu_over_a_folder_row();

        let _ = update_internal(
            &mut app,
            containers_intent(ContainersIntent::Run(
                RunIntent::FolderChecked {
                    path: PathBuf::from(FOLDER),
                    exists: false,
                },
            )),
        );

        assert!(app.widgets.modes.is_menu_open());
        assert_eq!(
            row_problem(&app),
            Some(i18n::t(i18n::Key::ContainersRunErrSourceMissing))
        );
    }

    #[test]
    fn a_typing_pause_under_an_open_menu_still_asks_about_the_folder() {
        let mut app = app_with_modes_menu_over_a_folder_row();

        let asked = collect_outputs(update_internal(
            &mut app,
            containers_intent(ContainersIntent::Run(
                RunIntent::FolderTypingPaused(PathBuf::from(FOLDER)),
            )),
        ));

        assert!(app.widgets.modes.is_menu_open());
        assert!(asked.iter().any(|event| matches!(
            event,
            AppEvent::Containers(ContainersEvent::Effect(
                ContainersEffect::Run(RunEffect::CheckFolder(path))
            )) if path.as_path() == Path::new(FOLDER)
        )));
    }

    #[test]
    fn a_late_answer_under_an_open_menu_still_shows_the_hint() {
        let mut app = app_with_modes_menu_over_a_folder_row();
        let _ = update_internal(
            &mut app,
            containers_intent(ContainersIntent::Run(
                RunIntent::FolderTypingPaused(PathBuf::from(FOLDER)),
            )),
        );
        assert_eq!(row_problem(&app), None);

        let _ = update_internal(
            &mut app,
            containers_intent(ContainersIntent::Run(
                RunIntent::FolderAnswerLate(PathBuf::from(FOLDER)),
            )),
        );

        assert!(app.widgets.modes.is_menu_open());
        assert_eq!(
            row_problem(&app),
            Some(i18n::t(i18n::Key::ContainersRunErrSourceNotAnswering))
        );
    }

    #[test]
    fn a_folder_answered_under_an_open_menu_is_asked_about_again() {
        let mut app = app_with_modes_menu_over_a_folder_row();
        let _ = update_internal(
            &mut app,
            containers_intent(ContainersIntent::Run(
                RunIntent::FolderChecked {
                    path: PathBuf::from(FOLDER),
                    exists: true,
                },
            )),
        );

        let asked = collect_outputs(update_internal(
            &mut app,
            containers_intent(ContainersIntent::Run(RunIntent::RecheckFolders)),
        ));

        assert!(matches!(
            asked.as_slice(),
            [AppEvent::Containers(ContainersEvent::Effect(
                ContainersEffect::Run(RunEffect::CheckFolder(path))
            ))] if path.as_path() == Path::new(FOLDER)
        ));
    }

    /// Run input from the user and the events its tasks produce, as the iced
    /// runtime does. Only for events that end in `Task::done`.
    fn update_to_the_end(app: &mut App, event: AppEvent) {
        for next in collect_outputs(update_input(app, event)) {
            internal_to_the_end(app, next);
        }
    }

    /// Run an event that does not come from the user and what follows from it.
    fn internal_to_the_end(app: &mut App, event: AppEvent) {
        for next in collect_outputs(update_internal(app, event)) {
            internal_to_the_end(app, next);
        }
    }

    /// The fixture lists of one engine are loaded and the right panel is
    /// closed.
    fn load_rows_and_close_right_panel(app: &mut App) {
        let resources = fixtures::resources(EngineKind::Docker);
        let containers = &mut app.widgets.containers;

        let _ = containers.reduce(ContainersIntent::Enter);
        let _ = containers.reduce(ContainersIntent::DiscoveryFinished(
            fixtures::discovery(&[EngineKind::Docker]),
        ));
        let _ = containers.reduce(ContainersIntent::ListsLoaded {
            engine: fixtures::engine(EngineKind::Docker).address().clone(),
            lists: LoadedLists {
                containers: Ok(resources.containers().to_vec()),
                images: Ok(resources.images().to_vec()),
                volumes: Ok(resources.volumes().to_vec()),
                networks: Ok(resources.networks().to_vec()),
            },
        });

        if app.widgets.sidebar.vm().is_right_open {
            let _ = app.widgets.sidebar.reduce(SidebarIntent::ToggleRight);
        }
    }

    fn start_inline_edit(app: &mut App) {
        let _ = update_input(
            app,
            AppEvent::QuickLaunch(QuickLaunchEvent::Intent(
                QuickLaunchIntent::HeaderCreateFolder,
            )),
        );
        assert!(app.widgets.quick_launch.has_inline_edit());
    }

    /// The fixture lists are loaded and the Logs tab of the first container is
    /// open, on an app where nothing hides the right panel.
    fn app_with_open_logs() -> App {
        let (mut app, _) = App::new();
        load_rows_and_close_right_panel(&mut app);
        let _ = app.widgets.sidebar.reduce(SidebarIntent::ToggleRight);
        update_to_the_end(
            &mut app,
            containers_intent(ContainersIntent::SelectInspectorTab(
                InspectorTab::Logs,
            )),
        );
        assert!(app.widgets.sidebar.vm().is_right_open);
        assert!(is_reading_logs(&app));

        app
    }

    fn sidebar_intent(intent: SidebarIntent) -> AppEvent {
        AppEvent::Sidebar(SidebarEvent::Intent(intent))
    }

    /// Whether a logs stream runs: the engine watch is one subscription, the
    /// logs the other.
    fn is_reading_logs(app: &App) -> bool {
        app.widgets.containers.subscription().units() == 2
    }

    /// How many lines the open logs show.
    fn log_lines(app: &App) -> usize {
        app.widgets.containers.log_line_count()
    }

    /// One line of the first fixture container comes in.
    fn push_a_log_line(app: &mut App) {
        let resources = fixtures::resources(EngineKind::Docker);
        let target = LogTarget::new(
            fixtures::engine(EngineKind::Docker),
            resources.containers()[0].id(),
        );

        internal_to_the_end(
            app,
            containers_intent(ContainersIntent::Inspector(
                InspectorIntent::LogLines {
                    target,
                    lines: vec![String::from("ready")],
                    position: None,
                },
            )),
        );
    }

    /// Run `event` and what follows from it. Returns whether logs were read
    /// after each update, in order.
    fn logs_after_each_update(app: &mut App, event: AppEvent) -> Vec<bool> {
        let follow_ups = collect_outputs(update_input(app, event));
        let mut reading = vec![is_reading_logs(app)];

        for next in follow_ups {
            reading.extend(logs_after_each_internal(app, next));
        }

        reading
    }

    fn logs_after_each_internal(app: &mut App, event: AppEvent) -> Vec<bool> {
        let follow_ups = collect_outputs(update_internal(app, event));
        let mut reading = vec![is_reading_logs(app)];

        for next in follow_ups {
            reading.extend(logs_after_each_internal(app, next));
        }

        reading
    }

    /// The right panel goes away by `hide`: the stream stops and the lines go.
    /// It comes back by `show`: one new stream starts with an empty view.
    fn assert_logs_follow_the_panel(hide: AppEvent, show: AppEvent) {
        let mut app = app_with_open_logs();
        push_a_log_line(&mut app);
        assert_eq!(log_lines(&app), 1);

        update_to_the_end(&mut app, hide);

        assert!(!app.widgets.sidebar.vm().is_right_open);
        assert!(!is_reading_logs(&app));

        update_to_the_end(&mut app, show);

        assert!(app.widgets.sidebar.vm().is_right_open);
        assert!(is_reading_logs(&app));
        assert_eq!(log_lines(&app), 0);
    }

    #[test]
    fn a_panel_toggle_stops_the_logs_and_starts_them_again() {
        assert_logs_follow_the_panel(
            sidebar_intent(SidebarIntent::ToggleRight),
            sidebar_intent(SidebarIntent::ToggleRight),
        );
    }

    #[test]
    fn the_files_panel_shortcut_stops_the_logs_and_starts_them_again() {
        assert_logs_follow_the_panel(
            shortcut_press(AppShortcut::FilesPanel),
            shortcut_press(AppShortcut::FilesPanel),
        );
    }

    #[test]
    fn the_action_bar_button_stops_the_logs_and_starts_them_again() {
        let press = || {
            AppEvent::Chrome(ChromeEvent::Intent(
                ChromeIntent::ToggleFilesPanel,
            ))
        };

        assert_logs_follow_the_panel(press(), press());
    }

    #[test]
    fn focus_mode_stops_the_logs_and_leaving_it_starts_them_again() {
        assert_logs_follow_the_panel(
            sidebar_intent(SidebarIntent::ToggleFocusMode),
            sidebar_intent(SidebarIntent::ToggleFocusMode),
        );
    }

    #[test]
    fn the_focus_mode_shortcut_stops_the_logs_and_starts_them_again() {
        assert_logs_follow_the_panel(
            shortcut_press(AppShortcut::FocusMode),
            shortcut_press(AppShortcut::FocusMode),
        );
    }

    #[test]
    fn the_left_panel_toggle_leaving_focus_mode_starts_the_logs_again() {
        assert_logs_follow_the_panel(
            sidebar_intent(SidebarIntent::ToggleFocusMode),
            sidebar_intent(SidebarIntent::ToggleLeft),
        );
    }

    #[test]
    fn revealing_the_launches_leaving_focus_mode_starts_the_logs_again() {
        assert_logs_follow_the_panel(
            sidebar_intent(SidebarIntent::ToggleFocusMode),
            sidebar_intent(SidebarIntent::RevealLaunches),
        );
    }

    #[test]
    fn revealing_the_right_panel_starts_the_logs_again() {
        assert_logs_follow_the_panel(
            sidebar_intent(SidebarIntent::ToggleRight),
            sidebar_intent(SidebarIntent::RevealRight),
        );
    }

    /// A drag of the edge of the right panel to `ratio` of the area right of
    /// the left panel.
    fn right_edge_drag(app: &App, ratio: f32) -> AppEvent {
        let pane_grid::Node::Split { b, .. } =
            app.widgets.sidebar.panes().layout()
        else {
            panic!("sidebar should split the left panel from the rest");
        };
        let pane_grid::Node::Split { id: split, .. } = b.as_ref() else {
            panic!("sidebar should split the main area from the right panel");
        };

        sidebar_intent(SidebarIntent::Resized(pane_grid::ResizeEvent {
            split: *split,
            ratio,
        }))
    }

    #[test]
    fn dragging_the_panel_shut_and_open_stops_and_starts_the_logs() {
        let app = app_with_open_logs();

        assert_logs_follow_the_panel(
            right_edge_drag(&app, 1.0),
            right_edge_drag(&app, 0.7),
        );
    }

    #[test]
    fn a_panel_closed_in_another_mode_keeps_the_logs_off_on_return() {
        let mut app = app_with_open_logs();
        internal_to_the_end(
            &mut app,
            containers_intent(ContainersIntent::Leave),
        );
        update_to_the_end(&mut app, sidebar_intent(SidebarIntent::ToggleRight));

        let _ = update_internal(
            &mut app,
            containers_intent(ContainersIntent::Enter),
        );
        let after_enter = is_reading_logs(&app);
        update_to_the_end(&mut app, sidebar_intent(SidebarIntent::ToggleRight));

        assert!(!after_enter);
        assert!(is_reading_logs(&app));
    }

    /// The row press selects the row first and the panel opens two updates
    /// later: the stream starts once, at the end, and never stops again.
    #[test]
    fn a_row_press_that_opens_the_panel_on_the_logs_tab_starts_one_stream() {
        let mut app = app_with_open_logs();
        update_to_the_end(&mut app, sidebar_intent(SidebarIntent::ToggleRight));

        let reading = logs_after_each_update(
            &mut app,
            containers_intent(ContainersIntent::SelectRow(1)),
        );

        let starts = reading.windows(2).filter(|step| !step[0] && step[1]);
        let stops = reading.windows(2).filter(|step| step[0] && !step[1]);
        assert!(app.widgets.sidebar.vm().is_right_open);
        assert_eq!(starts.count(), 1, "{reading:?}");
        assert_eq!(stops.count(), 0, "{reading:?}");
        assert_eq!(reading.last(), Some(&true), "{reading:?}");
    }

    #[test]
    fn an_update_that_leaves_the_panel_alone_sends_containers_nothing_new() {
        let mut app = app_with_open_logs();
        push_a_log_line(&mut app);

        let follow_ups = collect_outputs(update_internal(
            &mut app,
            AppEvent::SyncTerminalGridSizes,
        ));

        assert_eq!(log_lines(&app), 1);
        assert!(!follow_ups.iter().any(|event| matches!(
            event,
            AppEvent::Containers(ContainersEvent::Intent(_))
        )));
    }

    fn tabs_intent(intent: TabsIntent) -> AppEvent {
        AppEvent::Tabs(TabsEvent::Intent(intent))
    }

    fn chrome_intent(intent: ChromeIntent) -> AppEvent {
        AppEvent::Chrome(ChromeEvent::Intent(intent))
    }

    fn command_tab() -> AppEvent {
        tabs_intent(TabsIntent::OpenCommandTab {
            title: String::from("ssh"),
            settings: Box::new(Settings::default()),
            exit_behavior: ExitBehavior::Close,
            machine: TabMachine::ThisComputer,
        })
    }

    #[test]
    fn a_tab_opened_under_an_open_menu_keeps_its_terminal() {
        let mut app = app_with_open_modes_menu();

        internal_to_the_end(&mut app, command_tab());

        assert_eq!(app.widgets.tabs.len(), 1);
        assert_eq!(app.widgets.terminal_workspace.tabs().count(), 1);
        assert!(app.widgets.modes.is_menu_open());
    }

    fn focus_active() -> AppEvent {
        AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
            TerminalWorkspaceIntent::FocusActive,
        ))
    }

    /// Scenario 13: a palette dismiss or a mode switch asks for the active
    /// pane while the SSH prompt is open. The typed secret must stay in the
    /// prompt, so the pane does not take the keyboard.
    #[test]
    fn focus_active_under_the_secret_prompt_moves_no_focus() {
        let (mut app, _) = App::new();
        let _ = open_shell_tab(&mut app);
        let focused_without_the_prompt =
            runs_widget_operation(update_internal(&mut app, focus_active()));
        open_secret_prompt(&mut app);

        let task = update_internal(&mut app, focus_active());

        assert!(focused_without_the_prompt);
        assert!(!runs_widget_operation(task));
    }

    /// A tab that comes to the front in another mode shows Terminal mode,
    /// and the All modes menu closes as the mode switches (owner, Q2).
    #[test]
    fn a_tab_opened_in_containers_mode_shows_terminal_mode() {
        let mut app = app_with_open_modes_menu();
        let _ = app
            .widgets
            .modes
            .reduce(ModesIntent::Select(Mode::Containers));
        let _ = app.widgets.modes.reduce(ModesIntent::ToggleMenu);

        internal_to_the_end(&mut app, tabs_intent(open_launch_editor_intent()));

        assert_eq!(app.widgets.tabs.len(), 1);
        assert_eq!(app.widgets.modes.active(), Mode::Terminal);
        assert!(!app.widgets.modes.is_menu_open());
    }

    #[test]
    fn a_tab_closed_under_an_open_menu_lets_go_of_its_terminal() {
        let mut app = app_with_open_modes_menu();
        let _ = update_input(
            &mut app,
            AppEvent::Modes(ModesEvent::Intent(ModesIntent::CloseMenu)),
        );
        internal_to_the_end(&mut app, command_tab());
        let tab_id = app.widgets.tabs.active_tab_id().expect("a tab is open");
        let _ = update_input(
            &mut app,
            AppEvent::Modes(ModesEvent::Intent(ModesIntent::ToggleMenu)),
        );

        internal_to_the_end(
            &mut app,
            tabs_intent(TabsIntent::CloseTab { tab_id }),
        );

        assert_eq!(app.widgets.tabs.len(), 0);
        assert_eq!(app.widgets.terminal_workspace.tabs().count(), 0);
        assert!(app.widgets.modes.is_menu_open());
    }

    #[test]
    fn a_panel_button_under_an_open_menu_only_closes_the_menu() {
        let presses = [
            ChromeIntent::ToggleLeftPanel,
            ChromeIntent::ToggleFilesPanel,
            ChromeIntent::ToggleFocusMode,
        ];

        for press in presses {
            let mut app = app_with_open_modes_menu();
            let panels = app.widgets.sidebar.vm();

            update_to_the_end(&mut app, chrome_intent(press));

            let after = app.widgets.sidebar.vm();
            assert!(!app.widgets.modes.is_menu_open());
            assert_eq!(after.is_left_open, panels.is_left_open);
            assert_eq!(after.is_right_open, panels.is_right_open);
        }
    }

    #[test]
    fn the_palette_button_under_an_open_menu_only_closes_the_menu() {
        let mut app = app_with_open_modes_menu();

        update_to_the_end(
            &mut app,
            chrome_intent(ChromeIntent::OpenCommandPalette),
        );

        assert!(!app.widgets.modes.is_menu_open());
        assert!(!app.widgets.palette.is_open());
    }

    #[test]
    fn a_tab_press_under_an_open_menu_closes_it_and_switches_the_tab() {
        let (mut app, _) = App::new();
        app.widgets.modes = ModesWidget::new();
        internal_to_the_end(&mut app, command_tab());
        let first = app.widgets.tabs.active_tab_id().expect("a tab is open");
        internal_to_the_end(&mut app, command_tab());
        let _ = update_input(
            &mut app,
            AppEvent::Modes(ModesEvent::Intent(ModesIntent::ToggleMenu)),
        );

        // Only the first update: a terminal command further on needs the iced
        // runtime, and `collect_outputs` would wait for it for ever.
        let _ = update_input(
            &mut app,
            tabs_intent(TabsIntent::ActivateTab { tab_id: first }),
        );

        assert!(!app.widgets.modes.is_menu_open());
        assert_eq!(app.widgets.tabs.active_tab_id(), Some(first));
    }

    /// The open palette with `query` typed in it.
    fn app_with_palette_query(query: &str) -> App {
        let mut app = app_in(Mode::Terminal);
        open_palette(&mut app);
        let _ = update_input(
            &mut app,
            AppEvent::Palette(PaletteEvent::Intent(PaletteIntent::Action(
                PaletteAction::QueryChanged(query.to_string()),
            ))),
        );

        app
    }

    fn palette_query(app: &App) -> Option<&str> {
        app.widgets.palette.vm().map(|vm| vm.list.query())
    }

    /// Scenario 8: the palette is a menu, so a press on a header button
    /// closes it first.
    #[test]
    fn a_panel_button_while_the_palette_is_open_only_closes_it() {
        let presses = [
            ChromeIntent::ToggleLeftPanel,
            ChromeIntent::ToggleFilesPanel,
            ChromeIntent::ToggleFocusMode,
        ];

        for press in presses {
            let mut app = app_with_palette_query("");
            let panels = app.widgets.sidebar.vm();

            update_to_the_end(&mut app, chrome_intent(press));

            let after = app.widgets.sidebar.vm();
            assert!(!app.widgets.palette.is_open());
            assert_eq!(after.is_left_open, panels.is_left_open);
            assert_eq!(after.is_right_open, panels.is_right_open);
            assert_eq!(after.is_focus_mode, panels.is_focus_mode);
        }
    }

    #[test]
    fn the_search_button_while_the_palette_is_open_closes_it() {
        let mut app = app_with_palette_query("spl");

        update_to_the_end(
            &mut app,
            chrome_intent(ChromeIntent::OpenCommandPalette),
        );

        assert!(!app.widgets.palette.is_open());
    }

    /// Run input from the user and everything that follows from it, but
    /// not the effects of the settings widget: they would read or write the
    /// real settings file.
    fn update_without_settings_files(app: &mut App, event: AppEvent) {
        let mut queue = collect_outputs(update_input(app, event));
        while !queue.is_empty() {
            let event = queue.remove(0);
            if matches!(event, AppEvent::Settings(SettingsEvent::Effect(_))) {
                continue;
            }
            queue.extend(collect_outputs(update_internal(app, event)));
        }
    }

    #[test]
    fn the_settings_button_while_the_palette_is_open_closes_it_and_opens_settings()
     {
        let mut app = app_with_palette_query("");

        update_without_settings_files(
            &mut app,
            chrome_intent(ChromeIntent::OpenSettings),
        );

        assert!(!app.widgets.palette.is_open());
        assert!(app.widgets.settings.is_open());
        assert_eq!(app.widgets.tabs.len(), 0);
    }

    #[test]
    fn a_window_button_leaves_the_palette_open() {
        let presses = [
            ChromeIntent::MinimizeWindow,
            ChromeIntent::ToggleMaximize,
            ChromeIntent::PressTitleBar,
            ChromeIntent::MoveOnTitleBar,
            ChromeIntent::ReleaseTitleBar,
        ];

        for press in presses {
            let mut app = app_with_palette_query("spl");

            // Only the first update: moving the window needs the iced
            // runtime.
            let _ = update_input(&mut app, chrome_intent(press));

            assert_eq!(palette_query(&app), Some("spl"));
        }
    }

    #[test]
    fn a_shortcut_while_the_palette_is_open_closes_it_and_runs() {
        let mut app = app_with_palette_query("spl");

        update_to_the_end(&mut app, shortcut_press(AppShortcut::FocusMode));

        assert!(!app.widgets.palette.is_open());
        assert!(app.widgets.sidebar.vm().is_focus_mode);
    }

    /// Q1 (owner): the palette shortcut while the palette is open keeps it
    /// and what was typed in it.
    #[test]
    fn the_palette_shortcut_while_the_palette_is_open_keeps_it_and_its_query() {
        let mut app = app_with_palette_query("spl");

        update_to_the_end(
            &mut app,
            shortcut_press(AppShortcut::CommandPalette),
        );

        assert_eq!(palette_query(&app), Some("spl"));
    }

    #[test]
    fn typing_in_the_palette_keeps_it_open() {
        let app = app_with_palette_query("spl");

        assert_eq!(palette_query(&app), Some("spl"));
    }

    #[test]
    fn a_shortcut_under_the_row_menu_closes_it_and_runs() {
        let mut app = containers_app_with_lists();
        open_row_menu(&mut app);

        update_to_the_end(&mut app, shortcut_press(AppShortcut::FocusMode));

        assert!(!app.widgets.containers.has_row_menu());
        assert!(app.widgets.sidebar.vm().is_focus_mode);
    }

    #[test]
    fn a_row_menu_item_runs_its_action() {
        let mut app = containers_app_with_lists();
        open_row_menu(&mut app);

        let _ = update_input(
            &mut app,
            containers_intent(ContainersIntent::Actions(ActionsIntent::Act(
                RowAction::Remove,
            ))),
        );

        assert!(!app.widgets.containers.has_row_menu());
        assert!(app.widgets.containers.has_confirm());
    }

    fn quick_launch_intent(intent: QuickLaunchIntent) -> AppEvent {
        AppEvent::QuickLaunch(QuickLaunchEvent::Intent(intent))
    }

    /// Scenario 13: a shortcut under the SSH prompt does nothing, so no
    /// terminal behind the prompt takes the typed secret.
    #[test]
    fn a_shortcut_does_nothing_while_the_secret_prompt_is_open() {
        let mut app = app_in(Mode::Terminal);
        open_secret_prompt(&mut app);

        update_to_the_end(&mut app, shortcut_press(AppShortcut::FocusMode));

        assert!(app.widgets.quick_launch.has_secret_prompt());
        assert!(!app.widgets.sidebar.vm().is_focus_mode);
    }

    /// Scenario 11: a mode shortcut does not hide a Containers dialog that
    /// waits for its answer.
    #[test]
    fn a_mode_shortcut_does_nothing_while_a_containers_dialog_is_open() {
        let mut app = containers_app_with_lists();
        open_confirm(&mut app);

        update_to_the_end(&mut app, shortcut_press(AppShortcut::TerminalMode));

        assert_eq!(app.widgets.modes.active(), Mode::Containers);
        assert_eq!(top_overlay(&app), Some(Overlay::ContainersConfirm));
    }

    #[test]
    fn a_panel_button_does_nothing_while_the_secret_prompt_is_open() {
        let mut app = app_in(Mode::Terminal);
        open_secret_prompt(&mut app);
        let panels = app.widgets.sidebar.vm();

        update_to_the_end(
            &mut app,
            chrome_intent(ChromeIntent::ToggleLeftPanel),
        );

        let after = app.widgets.sidebar.vm();
        assert!(app.widgets.quick_launch.has_secret_prompt());
        assert_eq!(after.is_left_open, panels.is_left_open);
    }

    /// Scenario 14: an SSH prompt that comes up over an open menu takes
    /// what the user types.
    #[test]
    fn the_secret_prompt_takes_typing_under_an_open_menu() {
        let mut app = app_in(Mode::Terminal);
        open_header_menu(&mut app);
        open_secret_prompt(&mut app);

        let _ = update_input(
            &mut app,
            quick_launch_intent(QuickLaunchIntent::SecretPromptChanged(
                String::from("pw"),
            )),
        );

        assert_eq!(app.widgets.quick_launch.secret_prompt_value(), Some("pw"));
        assert!(app.widgets.sidebar.has_header_menu_open());
    }

    #[test]
    fn the_menu_under_a_cancelled_secret_prompt_is_still_open() {
        let mut app = app_in(Mode::Terminal);
        open_header_menu(&mut app);
        open_secret_prompt(&mut app);

        let _ = update_input(
            &mut app,
            quick_launch_intent(QuickLaunchIntent::SecretPromptCancel),
        );

        assert!(!app.widgets.quick_launch.has_secret_prompt());
        assert_eq!(top_overlay(&app), Some(Overlay::HeaderMenu));
    }

    #[test]
    fn an_app_shortcut_during_an_inline_edit_cancels_it() {
        let (mut app, _) = App::new();
        start_inline_edit(&mut app);

        update_to_the_end(&mut app, shortcut_press(AppShortcut::FilesPanel));

        assert!(!app.widgets.quick_launch.has_inline_edit());
    }

    #[test]
    fn a_plain_key_during_an_inline_edit_keeps_it() {
        let (mut app, _) = App::new();
        start_inline_edit(&mut app);
        let key = AppEvent::Keyboard(Event::KeyPressed {
            key: Key::Character("a".into()),
            modified_key: Key::Character("a".into()),
            physical_key: Physical::Code(Code::KeyA),
            location: Location::Standard,
            modifiers: Modifiers::empty(),
            text: Some("a".into()),
            repeat: false,
        });

        update_to_the_end(&mut app, key);

        assert!(app.widgets.quick_launch.has_inline_edit());
    }

    fn files_row_hovered() -> AppEvent {
        AppEvent::Explorer(ExplorerEvent::Intent(ExplorerIntent::NodeHovered {
            path: Some(vec![String::from("src")]),
        }))
    }

    #[test]
    fn hovering_a_files_row_during_an_inline_edit_keeps_it() {
        let (mut app, _) = App::new();
        start_inline_edit(&mut app);

        let _ = update_input(&mut app, files_row_hovered());

        assert!(app.widgets.quick_launch.has_inline_edit());
    }

    #[test]
    fn moving_over_the_title_bar_during_an_inline_edit_keeps_it() {
        let (mut app, _) = App::new();
        start_inline_edit(&mut app);

        let _ =
            update_input(&mut app, chrome_intent(ChromeIntent::MoveOnTitleBar));

        assert!(app.widgets.quick_launch.has_inline_edit());
    }

    #[test]
    fn hovering_a_files_row_under_an_open_menu_keeps_the_menu() {
        let mut app = app_with_open_modes_menu();

        let _ = update_input(&mut app, files_row_hovered());

        assert!(app.widgets.modes.is_menu_open());
    }

    #[test]
    fn the_settings_button_under_an_open_menu_closes_it_and_opens_settings() {
        let mut app = app_with_open_modes_menu();

        update_without_settings_files(
            &mut app,
            chrome_intent(ChromeIntent::OpenSettings),
        );

        assert!(!app.widgets.modes.is_menu_open());
        assert!(app.widgets.settings.is_open());
        assert_eq!(app.widgets.tabs.len(), 0);
    }

    /// Settings is a popup over every mode: it opens in Containers mode
    /// without a switch to Terminal mode and without a tab.
    #[test]
    fn the_settings_shortcut_in_containers_opens_the_popup_and_keeps_the_mode()
    {
        let mut app = containers_app_with_lists();

        update_without_settings_files(
            &mut app,
            shortcut_press(AppShortcut::Settings),
        );

        assert!(app.widgets.settings.is_open());
        assert_eq!(app.widgets.modes.active(), Mode::Containers);
        assert_eq!(app.widgets.tabs.len(), 0);
        assert_eq!(top_overlay(&app), Some(Overlay::Settings));
    }

    /// Owner, Q1 variant A: in Terminal mode the active pane gets its keys
    /// back when Settings closes.
    #[test]
    fn closing_settings_gives_the_pane_its_keys_in_terminal_mode() {
        let (mut app, _) = App::new();
        let _ = open_shell_tab(&mut app);
        open_settings(&mut app);

        let closed = collect_outputs(update_input(&mut app, escape_press()));
        let after_close: Vec<AppEvent> = closed
            .into_iter()
            .flat_map(|event| collect_outputs(update_internal(&mut app, event)))
            .collect();
        let focuses_the_pane =
            runs_widget_operation(update_internal(&mut app, focus_active()));

        assert!(!app.widgets.settings.is_open());
        assert!(matches!(
            after_close.as_slice(),
            [AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
                TerminalWorkspaceIntent::FocusActive
            ))]
        ));
        assert!(focuses_the_pane);
    }

    #[test]
    fn a_shortcut_does_nothing_while_settings_is_open() {
        let mut app = app_in(Mode::Terminal);
        open_settings(&mut app);

        update_to_the_end(&mut app, shortcut_press(AppShortcut::FocusMode));

        assert!(app.widgets.settings.is_open());
        assert!(!app.widgets.sidebar.vm().is_focus_mode);
    }

    /// Owner, Q3: the Settings shortcut while Settings is open does nothing,
    /// so the edits stay and the file is not read again.
    #[test]
    fn the_settings_shortcut_while_settings_is_open_keeps_the_edits() {
        let mut app = app_in(Mode::Terminal);
        open_settings(&mut app);
        let _ = app
            .widgets
            .settings
            .reduce(SettingsIntent::ShellChanged(String::from("/bin/edited")));

        let follow_ups = collect_outputs(update_input(
            &mut app,
            shortcut_press(AppShortcut::Settings),
        ));

        assert!(follow_ups.is_empty());
        assert!(app.widgets.settings.is_open());
        assert!(app.widgets.settings.has_unsaved_edits());
    }

    #[test]
    fn an_internal_event_leaves_an_inline_edit_alone() {
        let (mut app, _) = App::new();
        start_inline_edit(&mut app);

        let _ = update_internal(
            &mut app,
            sidebar_intent(SidebarIntent::ToggleRight),
        );

        assert!(app.widgets.quick_launch.has_inline_edit());
    }

    #[test]
    fn everything_a_message_leads_to_is_internal() {
        let (mut app, _) = App::new();

        let messages =
            collect_outputs(app.update(Message::Input(chrome_intent(
                ChromeIntent::OpenSettings,
            ))));

        assert!(!messages.is_empty());
        assert!(
            messages
                .iter()
                .all(|message| matches!(message, Message::Internal(_)))
        );
    }

    #[test]
    fn a_row_press_under_an_open_menu_only_closes_the_menu() {
        let mut app = app_with_open_modes_menu();
        load_rows_and_close_right_panel(&mut app);

        update_to_the_end(
            &mut app,
            containers_intent(ContainersIntent::SelectRow(1)),
        );

        assert!(!app.widgets.modes.is_menu_open());
        assert!(!app.widgets.sidebar.vm().is_right_open);
    }

    #[test]
    fn a_row_press_during_an_inline_edit_cancels_it_and_opens_the_right_panel()
    {
        let (mut app, _) = App::new();
        load_rows_and_close_right_panel(&mut app);
        start_inline_edit(&mut app);

        update_to_the_end(
            &mut app,
            containers_intent(ContainersIntent::SelectRow(1)),
        );

        assert!(!app.widgets.quick_launch.has_inline_edit());
        assert!(app.widgets.sidebar.vm().is_right_open);
    }

    #[test]
    fn a_menu_opened_right_after_a_row_press_stays_and_the_panel_still_opens() {
        let (mut app, _) = App::new();
        app.widgets.modes = ModesWidget::new();
        load_rows_and_close_right_panel(&mut app);

        let follow_ups = collect_outputs(update_input(
            &mut app,
            containers_intent(ContainersIntent::SelectRow(1)),
        ));
        let _ = update_input(
            &mut app,
            AppEvent::Modes(ModesEvent::Intent(ModesIntent::ToggleMenu)),
        );
        for event in follow_ups {
            internal_to_the_end(&mut app, event);
        }

        assert!(app.widgets.modes.is_menu_open());
        assert!(app.widgets.sidebar.vm().is_right_open);
    }

    #[test]
    fn an_inline_edit_started_right_after_a_row_press_stays_and_the_panel_still_opens()
     {
        let (mut app, _) = App::new();
        load_rows_and_close_right_panel(&mut app);

        let follow_ups = collect_outputs(update_input(
            &mut app,
            containers_intent(ContainersIntent::SelectRow(1)),
        ));
        start_inline_edit(&mut app);
        for event in follow_ups {
            internal_to_the_end(&mut app, event);
        }

        assert!(app.widgets.quick_launch.has_inline_edit());
        assert!(app.widgets.sidebar.vm().is_right_open);
    }

    /// What a focused terminal reports for `shortcut`. Terminal 999 is in
    /// no tab: the app turns the report into a command before the terminal
    /// workspace could look it up.
    fn terminal_binding(shortcut: AppShortcut) -> AppEvent {
        AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
            TerminalWorkspaceIntent::Widget(
                otty_ui_term::Event::BindingActionDispatched {
                    id: 999,
                    action: shortcut,
                },
            ),
        ))
    }

    /// Like [`press`], for the report of a focused terminal.
    fn press_in_terminal(app: &mut App, shortcut: AppShortcut) {
        let follow_ups =
            collect_outputs(update_input(app, terminal_binding(shortcut)));

        for event in follow_ups {
            let _ = update_internal(app, event);
        }
    }

    fn plain_key_press() -> AppEvent {
        AppEvent::Keyboard(Event::KeyPressed {
            key: Key::Character("a".into()),
            modified_key: Key::Character("a".into()),
            physical_key: Physical::Code(Code::KeyA),
            location: Location::Standard,
            modifiers: Modifiers::empty(),
            text: Some("a".into()),
            repeat: false,
        })
    }

    fn is_split_focused_pane(event: &AppEvent) -> bool {
        matches!(
            event,
            AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
                TerminalWorkspaceIntent::SplitFocusedPane {
                    axis: pane_grid::Axis::Vertical,
                }
            ))
        )
    }

    fn app_with_open_header_menu() -> App {
        let (mut app, _) = App::new();
        let _ = update_input(
            &mut app,
            sidebar_intent(SidebarIntent::HeaderMenuOpen(HeaderMenu::Sessions)),
        );
        assert!(app.widgets.sidebar.has_header_menu_open());
        app
    }

    #[test]
    fn a_shortcut_key_becomes_its_command() {
        let event = as_command(shortcut_press(AppShortcut::Settings));

        assert!(matches!(event, AppEvent::Command(AppCommand::OpenSettings)));
    }

    #[test]
    fn a_terminal_binding_becomes_its_command() {
        let event = as_command(terminal_binding(AppShortcut::PinnedMode2));

        assert!(matches!(
            event,
            AppEvent::Command(AppCommand::SelectModeAt { position: 2 })
        ));
    }

    #[test]
    fn a_plain_key_stays_a_key() {
        let event = as_command(plain_key_press());

        assert!(matches!(event, AppEvent::Keyboard(_)));
    }

    #[test]
    fn a_shortcut_key_from_the_subscription_runs_its_command() {
        let (mut app, _) = App::new();

        let events = collect_outputs(update_input(
            &mut app,
            shortcut_press(AppShortcut::Settings),
        ));

        assert!(events.iter().any(|event| matches!(
            event,
            AppEvent::Settings(SettingsEvent::Intent(SettingsIntent::Open))
        )));
    }

    #[test]
    fn a_terminal_binding_runs_its_command() {
        let (mut app, _) = App::new();

        let events = collect_outputs(update_input(
            &mut app,
            terminal_binding(AppShortcut::Settings),
        ));

        assert!(events.iter().any(|event| matches!(
            event,
            AppEvent::Settings(SettingsEvent::Intent(SettingsIntent::Open))
        )));
    }

    #[test]
    fn the_split_shortcut_splits_the_focused_pane_on_both_key_paths() {
        // From the keyboard subscription no terminal had keyboard focus;
        // the focused pane of the active tab is split all the same.
        for press in [
            shortcut_press(AppShortcut::SplitPane),
            terminal_binding(AppShortcut::SplitPane),
        ] {
            let (mut app, _) = App::new();
            let _ = open_shell_tab(&mut app);

            let events = collect_outputs(update_input(&mut app, press));

            assert!(events.iter().any(is_split_focused_pane));
        }
    }

    #[test]
    fn a_terminal_binding_during_an_inline_edit_cancels_it() {
        let (mut app, _) = App::new();
        start_inline_edit(&mut app);

        let _ =
            update_input(&mut app, terminal_binding(AppShortcut::FilesPanel));

        assert!(!app.widgets.quick_launch.has_inline_edit());
    }

    #[test]
    fn a_command_that_follows_from_input_leaves_an_inline_edit_alone() {
        // A palette entry runs its command this way, after the palette
        // already passed the checks.
        let (mut app, _) = App::new();
        start_inline_edit(&mut app);

        let _ = update_internal(
            &mut app,
            AppEvent::Command(AppCommand::ToggleFilesPanel),
        );

        assert!(app.widgets.quick_launch.has_inline_edit());
    }

    #[test]
    fn a_shortcut_under_an_open_menu_closes_it_and_runs_on_both_key_paths() {
        for press in [
            shortcut_press(AppShortcut::FocusMode),
            terminal_binding(AppShortcut::FocusMode),
        ] {
            let mut app = app_with_open_header_menu();

            update_to_the_end(&mut app, press);

            assert!(!app.widgets.sidebar.has_header_menu_open());
            assert!(app.widgets.sidebar.vm().is_focus_mode);
        }
    }

    #[test]
    fn a_mode_binding_from_a_terminal_switches_while_the_modes_menu_is_open() {
        let mut app = app_with_open_modes_menu();

        press_in_terminal(&mut app, AppShortcut::PinnedMode1);

        assert_eq!(app.widgets.modes.active(), Mode::Containers);
        assert!(!app.widgets.modes.is_menu_open());
    }
}

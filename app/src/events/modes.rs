//! Routes the modes widget: reduces its intents and runs its effects.
//! Saving the pinned modes and moving keyboard focus around the All modes
//! menu happen here, off the reducer.

use iced::Task;
use iced::widget::{Id, operation};

use super::AppEvent;
use crate::app::App;
use crate::widgets::modes::types::Mode;
use crate::widgets::modes::{ModesEffect, ModesEvent, storage};
use crate::widgets::terminal_workspace::{
    TerminalWorkspaceEvent, TerminalWorkspaceIntent,
};

/// Id no widget has. Focusing it takes keyboard focus away from every
/// widget, the focused terminal included.
const NO_WIDGET_ID: &str = "modes_menu_no_widget";

/// Route a modes event: reduce its intents and run its effects.
pub(crate) fn handle(app: &mut App, event: ModesEvent) -> Task<AppEvent> {
    match event {
        ModesEvent::Intent(intent) => {
            let was_menu_open = app.widgets.modes.is_menu_open();

            let reduced = app.widgets.modes.reduce(intent).map(AppEvent::Modes);

            let focus = focus_after_menu_change(app, was_menu_open);
            Task::batch([reduced, focus])
        },
        ModesEvent::Effect(effect) => handle_effect(effect),
    }
}

fn handle_effect(effect: ModesEffect) -> Task<AppEvent> {
    match effect {
        // The root view reads the active mode when it draws. Back in
        // Terminal mode the panes get the current grid size, since the
        // window may have changed meanwhile, and the active pane its keys.
        ModesEffect::ModeChanged(Mode::Terminal) => Task::batch([
            Task::done(AppEvent::SyncTerminalGridSizes),
            focus_active_terminal(),
        ]),
        ModesEffect::ModeChanged(_) => Task::none(),
        ModesEffect::SavePinned(pinned) => save_pinned(pinned),
    }
}

/// Move keyboard focus when the All modes menu opened or closed.
///
/// An open menu takes focus from the terminal, so typed keys do not reach
/// the shell and Esc reaches the app, which closes the menu. Once the menu
/// closes in Terminal mode, the active pane gets its keys back.
fn focus_after_menu_change(app: &App, was_menu_open: bool) -> Task<AppEvent> {
    let is_menu_open = app.widgets.modes.is_menu_open();

    if is_menu_open && !was_menu_open {
        return operation::focus(Id::new(NO_WIDGET_ID));
    }

    if was_menu_open
        && !is_menu_open
        && app.widgets.modes.active() == Mode::Terminal
    {
        return focus_active_terminal();
    }

    Task::none()
}

fn focus_active_terminal() -> Task<AppEvent> {
    Task::done(AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
        TerminalWorkspaceIntent::FocusActive,
    )))
}

/// Write the pinned modes to `modes.json` in the background. A failed
/// write is logged; the switcher keeps what the user chose.
fn save_pinned(pinned: Vec<Mode>) -> Task<AppEvent> {
    Task::perform(async move { storage::save_pinned(&pinned) }, |result| {
        if let Err(err) = result {
            log::warn!("saving the pinned modes failed: {err}");
        }
    })
    .discard()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::task_outputs::collect_outputs;
    use crate::widgets::modes::{ModesIntent, ModesWidget};
    use crate::widgets::tabs::{TabsCtx, TabsIntent};

    fn app() -> App {
        let (mut app, _) = App::new();
        app.widgets.modes = ModesWidget::new();
        app
    }

    #[test]
    fn an_intent_is_reduced_by_the_widget_and_its_effect_comes_back() {
        let mut app = app();

        let events = collect_outputs(handle(
            &mut app,
            ModesEvent::Intent(ModesIntent::Select(Mode::Docker)),
        ));

        assert!(matches!(
            events.as_slice(),
            [AppEvent::Modes(ModesEvent::Effect(
                ModesEffect::ModeChanged(Mode::Docker)
            ))]
        ));
        assert_eq!(app.widgets.modes.vm().active, Mode::Docker);
    }

    #[test]
    fn unpinning_asks_to_save_the_new_pinned_list() {
        let mut app = app();

        let events = collect_outputs(handle(
            &mut app,
            ModesEvent::Intent(ModesIntent::Unpin(Mode::Docker)),
        ));

        assert!(matches!(
            events.as_slice(),
            [AppEvent::Modes(ModesEvent::Effect(ModesEffect::SavePinned(
                pinned
            )))] if pinned.is_empty()
        ));
    }

    fn focuses_the_active_pane(event: &AppEvent) -> bool {
        matches!(
            event,
            AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
                TerminalWorkspaceIntent::FocusActive
            ))
        )
    }

    #[test]
    fn coming_back_to_terminal_syncs_the_grid_size_and_focuses_the_active_pane()
    {
        let mut app = app();

        let events = collect_outputs(handle(
            &mut app,
            ModesEvent::Effect(ModesEffect::ModeChanged(Mode::Terminal)),
        ));

        assert!(matches!(
            events.first(),
            Some(AppEvent::SyncTerminalGridSizes)
        ));
        assert!(events.iter().any(focuses_the_active_pane));
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn switching_to_docker_asks_nothing_of_the_terminals() {
        let mut app = app();

        let events = collect_outputs(handle(
            &mut app,
            ModesEvent::Effect(ModesEffect::ModeChanged(Mode::Docker)),
        ));

        assert!(events.is_empty());
    }

    #[test]
    fn switching_to_docker_and_back_keeps_the_tabs_and_the_active_tab() {
        let mut app = app();
        let ctx = TabsCtx { shell_name: "sh" };
        let _ = app.widgets.tabs.reduce(TabsIntent::OpenSettingsTab, &ctx);
        let _ = app.widgets.tabs.reduce(TabsIntent::OpenTerminalTab, &ctx);
        let tab_count = app.widgets.tabs.len();
        let active_tab = app.widgets.tabs.active_tab_id();

        let to_docker = collect_outputs(handle(
            &mut app,
            ModesEvent::Intent(ModesIntent::Select(Mode::Docker)),
        ));
        let in_docker =
            (app.widgets.tabs.len(), app.widgets.tabs.active_tab_id());
        let back = collect_outputs(handle(
            &mut app,
            ModesEvent::Intent(ModesIntent::Select(Mode::Terminal)),
        ));

        assert_eq!(to_docker.len(), 1);
        assert_eq!(in_docker, (tab_count, active_tab));
        assert!(matches!(
            back.as_slice(),
            [AppEvent::Modes(ModesEvent::Effect(
                ModesEffect::ModeChanged(Mode::Terminal)
            ))]
        ));
        assert_eq!(app.widgets.modes.active(), Mode::Terminal);
        assert_eq!(app.widgets.tabs.len(), tab_count);
        assert_eq!(app.widgets.tabs.active_tab_id(), active_tab);
    }

    #[test]
    fn opening_the_menu_reports_nothing_else() {
        let mut app = app();

        let events = collect_outputs(handle(
            &mut app,
            ModesEvent::Intent(ModesIntent::ToggleMenu),
        ));

        assert!(app.widgets.modes.is_menu_open());
        assert!(events.is_empty());
    }

    #[test]
    fn closing_the_menu_in_terminal_mode_gives_the_terminal_its_keys_back() {
        let mut app = app();
        let _ = handle(&mut app, ModesEvent::Intent(ModesIntent::ToggleMenu));

        let events = collect_outputs(handle(
            &mut app,
            ModesEvent::Intent(ModesIntent::CloseMenu),
        ));

        assert!(!app.widgets.modes.is_menu_open());
        assert!(events.iter().any(focuses_the_active_pane));
    }

    #[test]
    fn closing_the_menu_in_docker_mode_focuses_nothing() {
        let mut app = app();
        let _ = handle(
            &mut app,
            ModesEvent::Intent(ModesIntent::Select(Mode::Docker)),
        );
        let _ = handle(&mut app, ModesEvent::Intent(ModesIntent::ToggleMenu));

        let events = collect_outputs(handle(
            &mut app,
            ModesEvent::Intent(ModesIntent::CloseMenu),
        ));

        assert!(events.is_empty());
    }

    #[test]
    fn closing_a_closed_menu_focuses_nothing() {
        let mut app = app();

        let events = collect_outputs(handle(
            &mut app,
            ModesEvent::Intent(ModesIntent::CloseMenu),
        ));

        assert!(events.is_empty());
    }
}

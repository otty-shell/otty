use iced::Task;

use crate::app::App;
use crate::event::AppEvent;
use crate::widgets::modes::event::ModesIntent;

/// Deliver a modes intent to the widget's reducer.
pub(crate) fn deliver(app: &mut App, intent: ModesIntent) -> Task<AppEvent> {
    app.widgets.modes.reduce(intent).map(AppEvent::Modes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::mode::Mode;
    use crate::testing::task_outputs::collect_outputs;
    use crate::testing::window::open_launch_editor_intent;
    use crate::widgets::modes::ModesWidget;
    use crate::widgets::modes::event::{ModesEffect, ModesEvent};
    use crate::widgets::tabs::TabsCtx;
    use crate::widgets::tabs::event::TabsIntent;

    fn app() -> App {
        let (mut app, _) = App::new();
        app.widgets.modes = ModesWidget::new();
        app
    }

    #[test]
    fn an_intent_is_reduced_by_the_widget_and_its_effect_comes_back() {
        let mut app = app();

        let events = collect_outputs(deliver(
            &mut app,
            ModesIntent::Select(Mode::Containers),
        ));

        assert!(matches!(
            events.as_slice(),
            [AppEvent::Modes(ModesEvent::Effect(
                ModesEffect::ModeChanged(Mode::Containers)
            ))]
        ));
        assert_eq!(app.widgets.modes.vm().active, Mode::Containers);
    }

    #[test]
    fn unpinning_asks_to_save_the_new_pinned_list() {
        let mut app = app();

        let events = collect_outputs(deliver(
            &mut app,
            ModesIntent::Unpin(Mode::Containers),
        ));

        assert!(matches!(
            events.as_slice(),
            [AppEvent::Modes(ModesEvent::Effect(ModesEffect::SavePinned(
                pinned
            )))] if pinned.is_empty()
        ));
    }

    #[test]
    fn switching_to_containers_and_back_keeps_the_tabs_and_the_active_tab() {
        let mut app = app();
        let ctx = TabsCtx { shell_name: "sh" };
        let _ = app.widgets.tabs.reduce(open_launch_editor_intent(), &ctx);
        let _ = app.widgets.tabs.reduce(TabsIntent::OpenTerminalTab, &ctx);
        let tab_count = app.widgets.tabs.len();
        let active_tab = app.widgets.tabs.active_tab_id();

        let to_containers = collect_outputs(deliver(
            &mut app,
            ModesIntent::Select(Mode::Containers),
        ));
        let in_containers =
            (app.widgets.tabs.len(), app.widgets.tabs.active_tab_id());
        let back = collect_outputs(deliver(
            &mut app,
            ModesIntent::Select(Mode::Terminal),
        ));

        assert_eq!(to_containers.len(), 1);
        assert_eq!(in_containers, (tab_count, active_tab));
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
}

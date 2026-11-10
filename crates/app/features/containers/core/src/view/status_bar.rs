//! The left side of the Containers mode status bar: the engine and the counts
//! of the open list. The frame draws the bar and adds the sessions button and
//! the OTTY version on the right.

use otty_app_i18n::{self as i18n, Key};
use otty_ui_kit::components::status_bar::StatusItem;
use otty_ui_kit::components::target_dot::TargetKind;

use super::super::model::StatusBarViewModel;

/// Build the items of the left side. With an engine they show its name and
/// version, then the counts; without one they name the mode and give the
/// reason.
pub(crate) fn items<Message>(
    vm: &StatusBarViewModel,
) -> Vec<StatusItem<Message>> {
    let engine = match &vm.engine {
        Some(engine) => StatusItem::plain_target(TargetKind::Local, engine),
        None => StatusItem::text(i18n::t(Key::ContainersStatusMode)),
    };

    vec![engine, StatusItem::text(&vm.info)]
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use otty_app_theme::theme::ThemeManager;
    use otty_ui_kit::components::status_bar::StatusBar;

    use super::*;

    fn vm(engine: Option<&str>) -> StatusBarViewModel {
        StatusBarViewModel {
            engine: engine.map(String::from),
            info: String::from("6 containers, 3 running"),
        }
    }

    /// A status bar with `items` on its left side.
    fn bar(items: Vec<StatusItem<()>>) -> StatusBar<()> {
        let themes = ThemeManager::new();
        let mut bar = StatusBar::new(themes.current().ui());
        for item in items {
            bar = bar.left(item);
        }

        bar
    }

    #[test]
    fn with_an_engine_the_bar_shows_the_engine_and_the_counts() {
        let mut ui = simulator(bar(items(&vm(Some("Docker 27.3")))));

        for label in ["Docker 27.3", "6 containers, 3 running"] {
            ui.find(label).expect("status bar item should be visible");
        }
        assert!(ui.find(i18n::t(Key::ContainersStatusMode)).is_err());
    }

    #[test]
    fn without_an_engine_the_bar_names_the_mode() {
        let mut ui = simulator(bar(items(&vm(None))));

        ui.find(i18n::t(Key::ContainersStatusMode))
            .expect("mode name should be visible");
        ui.find("6 containers, 3 running")
            .expect("text should be visible");
    }
}

//! The left side of the Agents mode status bar: the counts of the projects
//! and their sessions. The frame draws the bar and adds the sessions button
//! and the OTTY version on the right.

use otty_ui_kit::components::status_bar::StatusItem;

use super::super::model::StatusBarViewModel;

/// Build the items of the left side, one text each.
pub(crate) fn items<Message>(
    vm: &StatusBarViewModel,
) -> Vec<StatusItem<Message>> {
    vm.items.iter().map(StatusItem::text).collect()
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use otty_app_theme::theme::ThemeManager;
    use otty_ui_kit::components::status_bar::StatusBar;

    use super::*;

    #[test]
    fn each_text_is_an_item_on_the_left() {
        let vm = StatusBarViewModel {
            items: vec![
                String::from("2 projects"),
                String::from("3 agent sessions"),
            ],
        };
        let themes = ThemeManager::new();
        let mut bar = StatusBar::<()>::new(themes.current().ui());
        for item in items(&vm) {
            bar = bar.left(item);
        }
        let mut ui = simulator(bar);

        ui.find("2 projects").expect("the projects should show");
        ui.find("3 agent sessions")
            .expect("the sessions should show");
    }
}

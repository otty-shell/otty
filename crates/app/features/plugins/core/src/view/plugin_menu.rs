//! The menu of a plugin in the left panel (issue 37), in the style of the
//! context menus of Files and Quick Launch: the actions of the plugin's
//! page, in its order, and each item sends the intent of its button.

use iced::Element;
use otty_app_i18n::{self as i18n, Key};
use otty_ui_kit::components::context_menu::context_menu;
use otty_ui_kit::components::menu::{Menu, MenuItem};
use otty_ui_kit::palette::UiPalette;

use crate::event::PluginsIntent;
use crate::model::PluginMenuViewModel;

/// Render the menu of a plugin at the pointer, kept inside the main area.
/// A press away from it closes it.
pub(crate) fn view<'a>(
    vm: PluginMenuViewModel,
    palette: &UiPalette,
) -> Element<'a, PluginsIntent> {
    let place = vm.place;

    context_menu(menu(vm, palette), place, PluginsIntent::CloseMenu)
}

/// Reload for a local plugin or Check for updates for one from a
/// repository, Review update… while an update waits, Show log, and
/// Remove…, which asks first. While a change runs the items that change
/// the plugin are disabled.
fn menu(vm: PluginMenuViewModel, palette: &UiPalette) -> Menu<PluginsIntent> {
    let id = vm.id;
    let changes = |label: &str, intent: PluginsIntent| {
        let item = MenuItem::new(label);
        if vm.is_busy {
            item
        } else {
            item.on_press(intent)
        }
    };

    let first = if vm.is_local {
        changes(
            i18n::t(Key::PluginsReload),
            PluginsIntent::Reload(id.clone()),
        )
    } else {
        changes(
            i18n::t(Key::PluginsCheckForUpdates),
            PluginsIntent::CheckForUpdates(id.clone()),
        )
    };
    let mut menu = Menu::new(palette).item(first);
    if vm.has_waiting_update {
        menu = menu.item(changes(
            i18n::t(Key::PluginsMenuReviewUpdate),
            PluginsIntent::Review(id.clone()),
        ));
    }
    let log = MenuItem::new(i18n::t(Key::PluginsShowLog))
        .on_press(PluginsIntent::ShowLog(id.clone()));
    let remove = changes(
        i18n::t(Key::PluginsMenuRemove),
        PluginsIntent::AskRemove(id),
    )
    .danger(true);

    menu.item(log).separator().item(remove)
}

#[cfg(test)]
mod tests {
    use iced::{Point, Size};
    use iced_test::simulator;
    use otty_ui_kit::components::context_menu::MenuPlace;

    use super::*;

    fn menu_of(
        is_local: bool,
        has_waiting_update: bool,
    ) -> PluginMenuViewModel {
        PluginMenuViewModel {
            id: String::from("example"),
            is_local,
            has_waiting_update,
            is_busy: false,
            place: MenuPlace::new(
                Point::new(40.0, 40.0),
                Size::new(1024.0, 768.0),
            ),
        }
    }

    /// Click the item `label` in `vm` and return what it sent.
    fn click(vm: PluginMenuViewModel, label: &str) -> Vec<PluginsIntent> {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(view(vm, &palette));

        ui.click(label).expect("the item should show");

        ui.into_messages().collect()
    }

    fn sent_one(intents: &[PluginsIntent]) -> &PluginsIntent {
        match intents {
            [intent] => intent,
            other => panic!("one intent: {other:?}"),
        }
    }

    /// Issue 37: a plugin from a repository checks for updates, shows its
    /// log and asks before it is removed, as its page does.
    #[test]
    fn a_repository_plugin_checks_for_updates_shows_its_log_and_removes() {
        let check =
            click(menu_of(false, false), i18n::t(Key::PluginsCheckForUpdates));
        let log = click(menu_of(false, false), i18n::t(Key::PluginsShowLog));
        let remove =
            click(menu_of(false, false), i18n::t(Key::PluginsMenuRemove));

        assert!(matches!(
            sent_one(&check),
            PluginsIntent::CheckForUpdates(id) if id == "example"
        ));
        assert!(matches!(
            sent_one(&log),
            PluginsIntent::ShowLog(id) if id == "example"
        ));
        assert!(matches!(
            sent_one(&remove),
            PluginsIntent::AskRemove(id) if id == "example"
        ));
    }

    /// A local plugin reloads in place of Check for updates.
    #[test]
    fn a_local_plugin_reloads() {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(view(menu_of(true, false), &palette));
        assert!(ui.find(i18n::t(Key::PluginsCheckForUpdates)).is_err());

        let reload = click(menu_of(true, false), i18n::t(Key::PluginsReload));

        assert!(matches!(
            sent_one(&reload),
            PluginsIntent::Reload(id) if id == "example"
        ));
    }

    /// Review update… shows only while an update waits.
    #[test]
    fn review_update_shows_only_while_an_update_waits() {
        let palette = UiPalette::otty_dark();
        let mut without = simulator(view(menu_of(false, false), &palette));
        assert!(without.find(i18n::t(Key::PluginsMenuReviewUpdate)).is_err());

        let review =
            click(menu_of(false, true), i18n::t(Key::PluginsMenuReviewUpdate));

        assert!(matches!(
            sent_one(&review),
            PluginsIntent::Review(id) if id == "example"
        ));
    }

    /// While a change runs the items that change the plugin send nothing;
    /// Show log still shows the log.
    #[test]
    fn while_a_change_runs_only_show_log_works() {
        let busy = || PluginMenuViewModel {
            is_busy: true,
            ..menu_of(false, true)
        };

        let pressed: Vec<PluginsIntent> = [
            Key::PluginsCheckForUpdates,
            Key::PluginsMenuReviewUpdate,
            Key::PluginsMenuRemove,
        ]
        .into_iter()
        .flat_map(|key| click(busy(), i18n::t(key)))
        .collect();
        let log = click(busy(), i18n::t(Key::PluginsShowLog));

        assert!(pressed.is_empty());
        assert!(matches!(sent_one(&log), PluginsIntent::ShowLog(_)));
    }

    /// A press away from the menu closes it.
    #[test]
    fn a_press_outside_the_menu_closes_it() {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(view(menu_of(false, false), &palette));

        ui.point_at(Point::new(900.0, 700.0));
        let _ = ui.simulate(iced_test::simulator::click());

        let intents: Vec<PluginsIntent> = ui.into_messages().collect();
        assert!(matches!(intents.as_slice(), [PluginsIntent::CloseMenu]));
    }
}

//! The left panel of Plugins mode under the switcher: one section of the
//! installed plugins, as the sections of Agents mode, with Add plugin…
//! and Add plugin from folder… in its header, the search field and a row
//! for each plugin whose name holds the search.

use iced::widget::{Column, Id, column, container, scrollable, svg};
use iced::{Element, Length, Padding};
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::icons;
use otty_ui_kit::components::icon_button::{
    IconButton, IconButtonSize, IconTone,
};
use otty_ui_kit::components::panel_section::PanelSection;
use otty_ui_kit::components::scrollbar;
use otty_ui_kit::components::search_field::SearchField;
use otty_ui_kit::components::small_text::small_text;
use otty_ui_kit::components::tree_row::TreeRow;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::spacing;

use crate::event::PluginsIntent;
use crate::model::{PluginListRowViewModel, SidePanelViewModel};

/// Widget id of the search field of the left panel.
pub(crate) const SEARCH_FIELD: Id = Id::new("plugins_search");

/// Render the left panel: the section of the installed plugins, which
/// takes the height of the panel; its rows scroll inside it.
pub(crate) fn view<'a>(
    vm: SidePanelViewModel<'a>,
    palette: &UiPalette,
) -> Element<'a, PluginsIntent> {
    let add = IconButton::new(
        svg::Handle::from_memory(icons::PLUS),
        i18n::t(Key::PluginsAddPlugin),
        palette,
    )
    .size(IconButtonSize::Small)
    .tone(IconTone::Accent)
    .on_press(PluginsIntent::AddPlugin);
    let add_folder = IconButton::new(
        svg::Handle::from_memory(icons::FOLDER),
        i18n::t(Key::PluginsAddFromFolder),
        palette,
    )
    .size(IconButtonSize::Small)
    .on_press(PluginsIntent::AddFromFolder);

    let search = SearchField::new(
        vm.search,
        svg::Handle::from_memory(icons::SEARCH),
        palette,
    )
    .placeholder(i18n::t(Key::PluginsSearchPlaceholder))
    .id(SEARCH_FIELD)
    .on_input(PluginsIntent::SearchChanged)
    .on_clear(
        PluginsIntent::SearchChanged(String::new()),
        i18n::t(Key::AgentsSearchClear),
    );

    let mut rows = Column::new().width(Length::Fill);
    for plugin in vm.rows {
        rows = rows.push(plugin_row(plugin, palette));
    }
    if let Some(note) = vm.note {
        rows = rows.push(
            container(small_text(note, palette.text_muted()))
                .padding(Padding::from([spacing::SPACE_1, spacing::SPACE_3])),
        );
    }
    let rows = scrollable(rows)
        .width(Length::Fill)
        .height(Length::Fill)
        .direction(scrollbar::vertical())
        .style(scrollbar::style(palette));

    PanelSection::new(i18n::t(Key::PluginsSectionInstalled), palette)
        .open(true)
        .grow(true)
        .count(vm.count)
        .action(add)
        .action(add_folder)
        .body(
            column![search, rows]
                .spacing(spacing::SPACE_2)
                .height(Length::Fill),
        )
        .into()
}

/// A plugin: its icon, its name, its mark and its sign. A press shows its
/// page.
fn plugin_row<'a>(
    vm: PluginListRowViewModel,
    palette: &UiPalette,
) -> Element<'a, PluginsIntent> {
    let note = vm.note();
    let tone = vm.tone();

    let row = TreeRow::new(vm.name.clone(), palette);
    let row = match vm.icon {
        Some(bytes) => row.icon(svg::Handle::from_memory(bytes), None),
        None => row.letter_icon(vm.name),
    };

    row.state(note)
        .count_tone(tone)
        .selected(vm.is_selected)
        .on_press(PluginsIntent::Select(vm.id))
        .into()
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;

    use super::*;
    use crate::model::PluginSign;

    fn example() -> PluginListRowViewModel {
        PluginListRowViewModel {
            id: String::from("example"),
            name: String::from("Example"),
            trust: "Official",
            icon: None,
            sign: Some(PluginSign::Failed),
            is_selected: false,
        }
    }

    fn panel(rows: Vec<PluginListRowViewModel>) -> SidePanelViewModel<'static> {
        SidePanelViewModel {
            search: "",
            count: rows.len(),
            rows,
            note: None,
        }
    }

    /// A press on a plugin selects it; its row shows its mark and sign.
    #[test]
    fn a_press_on_a_plugin_selects_it() {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(view(panel(vec![example()]), &palette));

        assert!(ui.find(example().note()).is_ok());
        let _ = ui.click("Example");

        let sent: Vec<PluginsIntent> = ui.into_messages().collect();
        assert!(matches!(
            sent.as_slice(),
            [PluginsIntent::Select(id)] if id == "example"
        ));
    }

    /// The search field sends what is typed.
    #[test]
    fn typing_in_the_search_field_searches() {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(view(panel(vec![example()]), &palette));

        let _ = ui.click(SEARCH_FIELD);
        let _ = ui.typewrite("ex");

        let sent: Vec<PluginsIntent> = ui.into_messages().collect();
        assert!(
            sent.iter().any(|intent| matches!(
                intent,
                PluginsIntent::SearchChanged(_)
            ))
        );
    }

    #[test]
    fn a_search_that_hides_every_plugin_says_so() {
        let palette = UiPalette::otty_dark();
        let mut vm = panel(Vec::new());
        vm.count = 1;
        vm.note = Some("No plugin matches the search");
        let mut ui = simulator(view(vm, &palette));

        assert!(ui.find("No plugin matches the search").is_ok());
    }
}

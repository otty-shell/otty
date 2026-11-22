//! The left panel of Plugins mode under the switcher: one section of the
//! installed plugins, as the sections of Agents mode, with the Add plugin
//! split button in its header, the search field and a row for each plugin
//! whose name holds the search. A right click on a row opens the menu of
//! its plugin.

use iced::widget::{
    Column, Id, column, container, mouse_area, scrollable, svg,
};
use iced::{Element, Length, Padding};
use otty_app_i18n::{self as i18n, Key};
use otty_app_slots::{SectionMessage, SectionState};
use otty_app_theme::icons;
use otty_ui_kit::components::panel_section::PanelSection;
use otty_ui_kit::components::scrollbar;
use otty_ui_kit::components::search_field::SearchField;
use otty_ui_kit::components::small_text::small_text;
use otty_ui_kit::components::split_button::SplitButton;
use otty_ui_kit::components::tree_row::TreeRow;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::spacing;

use crate::event::PluginsIntent;
use crate::model::{PluginListRowViewModel, SidePanelViewModel};

/// What a press in the left panel sends: the chevron of Add plugin belongs
/// to the frame, which keeps its menu; the rest to Plugins mode.
type Message = SectionMessage<PluginsIntent>;

/// Widget id of the search field of the left panel.
pub(crate) const SEARCH_FIELD: Id = Id::new("plugins_search");

/// Render the left panel: the section of the installed plugins, which
/// takes the height of the panel; its rows scroll inside it. The section
/// is always open; `section` tells whether the menu of Add plugin is open,
/// which frames its chevron.
pub(crate) fn view<'a>(
    vm: SidePanelViewModel<'a>,
    section: SectionState,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let add = SplitButton::new(
        svg::Handle::from_memory(icons::PLUS),
        i18n::t(Key::PluginsAddPlugin),
        i18n::t(Key::PluginsAddMenu),
        palette,
    )
    .menu_open(section.is_menu_open())
    .on_press(SectionMessage::Feature(PluginsIntent::AddPlugin))
    .on_menu(SectionMessage::OpenMenu);

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

    let body: Element<'a, PluginsIntent> = column![search, rows]
        .spacing(spacing::SPACE_2)
        .height(Length::Fill)
        .into();

    PanelSection::new(i18n::t(Key::PluginsSectionInstalled), palette)
        .open(true)
        .grow(true)
        .count(vm.count)
        .action(add)
        .body(body.map(SectionMessage::Feature))
        .into()
}

/// A plugin: its icon, its name, its mark and its sign. A press shows its
/// page; a right click opens its menu.
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
    let row = row
        .state(note)
        .count_tone(tone)
        .selected(vm.is_selected)
        .on_press(PluginsIntent::Select(vm.id.clone()));

    mouse_area(row)
        .on_right_press(PluginsIntent::OpenMenu(vm.id))
        .into()
}

#[cfg(test)]
mod tests {
    use iced::Point;
    use iced_test::simulator;
    use otty_ui_kit::size;

    use super::*;
    use crate::model::PluginSign;

    /// Width of the panel in the tests.
    const PANEL_WIDTH: f32 = 260.0;
    /// The plus of the split button, from the right edge of the panel.
    const PLUS_X: f32 =
        PANEL_WIDTH - 8.0 - size::SPLIT_CHEVRON - size::CONTROL_SM / 2.0;
    /// The chevron of the split button, from the right edge of the panel.
    const CHEVRON_X: f32 = PANEL_WIDTH - 8.0 - size::SPLIT_CHEVRON / 2.0;
    /// Section padding 8, header row 30 high: the middle of the header.
    const HEADER_Y: f32 = 8.0 + 15.0;

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

    fn closed_menu() -> SectionState {
        SectionState::new(true, false)
    }

    /// The messages of a press on the header at `x`.
    fn click_header_at(x: f32) -> Vec<Message> {
        let palette = UiPalette::otty_dark();
        let panel = container(view(panel(Vec::new()), closed_menu(), &palette))
            .width(PANEL_WIDTH);
        let mut ui = simulator(panel);

        ui.point_at(Point::new(x, HEADER_Y));
        let _ = ui.simulate(iced_test::simulator::click());

        ui.into_messages().collect()
    }

    /// The owner, 2026-11-22: a press on the plus adds a plugin from a
    /// repository, by its GitHub link.
    #[test]
    fn a_press_on_the_plus_adds_a_plugin_from_a_repository() {
        let sent = click_header_at(PLUS_X);

        assert!(matches!(
            sent.as_slice(),
            [SectionMessage::Feature(PluginsIntent::AddPlugin)]
        ));
    }

    /// The chevron opens the menu of the ways to add a plugin, which the
    /// frame keeps.
    #[test]
    fn a_press_on_the_chevron_opens_the_menu() {
        let sent = click_header_at(CHEVRON_X);

        assert!(matches!(sent.as_slice(), [SectionMessage::OpenMenu]));
    }

    /// A press on a plugin selects it; its row shows its mark and sign.
    #[test]
    fn a_press_on_a_plugin_selects_it() {
        let palette = UiPalette::otty_dark();
        let mut ui =
            simulator(view(panel(vec![example()]), closed_menu(), &palette));

        assert!(ui.find(example().note()).is_ok());
        let _ = ui.click("Example");

        let sent: Vec<Message> = ui.into_messages().collect();
        assert!(matches!(
            sent.as_slice(),
            [SectionMessage::Feature(PluginsIntent::Select(id))]
                if id == "example"
        ));
    }

    /// Issue 37: a right click on a plugin opens its menu and selects
    /// nothing.
    #[test]
    fn a_right_click_on_a_plugin_opens_its_menu() {
        let palette = UiPalette::otty_dark();
        let mut ui =
            simulator(view(panel(vec![example()]), closed_menu(), &palette));

        let row = ui.find("Example").expect("the row should show");
        ui.point_at(row.bounds().center());
        let _ = ui.simulate(otty_app_testing::mouse::right_click());

        let sent: Vec<Message> = ui.into_messages().collect();
        assert!(matches!(
            sent.as_slice(),
            [SectionMessage::Feature(PluginsIntent::OpenMenu(id))]
                if id == "example"
        ));
    }

    /// The search field sends what is typed.
    #[test]
    fn typing_in_the_search_field_searches() {
        let palette = UiPalette::otty_dark();
        let mut ui =
            simulator(view(panel(vec![example()]), closed_menu(), &palette));

        let _ = ui.click(SEARCH_FIELD);
        let _ = ui.typewrite("ex");

        let sent: Vec<Message> = ui.into_messages().collect();
        assert!(sent.iter().any(|message| matches!(
            message,
            SectionMessage::Feature(PluginsIntent::SearchChanged(_))
        )));
    }

    #[test]
    fn a_search_that_hides_every_plugin_says_so() {
        let palette = UiPalette::otty_dark();
        let mut vm = panel(Vec::new());
        vm.count = 1;
        vm.note = Some("No plugin matches the search");
        let mut ui = simulator(view(vm, closed_menu(), &palette));

        assert!(ui.find("No plugin matches the search").is_ok());
    }
}

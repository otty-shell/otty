//! The icon of an agent or a provider: the SVG of its registry entry,
//! tinted, or the first letter of its name on a rounded square when it has
//! none (spec 2026-11-19-agent-plugins, "Links in the interface").

use iced::widget::svg;
use iced::{Color, Element};
use otty_ui_kit::components::letter_icon::letter_icon;
use otty_ui_kit::components::tree_row::TreeRow;
use otty_ui_kit::palette::UiPalette;

use super::super::event::AgentsIntent;
use super::super::model::EntryIcon;

/// `icon` as an `edge` pixels square in `color`.
pub(super) fn view<'a>(
    icon: EntryIcon,
    edge: f32,
    color: Color,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    match icon {
        EntryIcon::Svg(bytes) => svg(svg::Handle::from_memory(bytes))
            .width(edge)
            .height(edge)
            .style(move |_, _| svg::Style { color: Some(color) })
            .into(),
        EntryIcon::Letter(name) => letter_icon(&name, edge, color, palette),
    }
}

/// `row` with `icon` before its label.
pub(super) fn in_row(
    row: TreeRow<AgentsIntent>,
    icon: EntryIcon,
) -> TreeRow<AgentsIntent> {
    match icon {
        EntryIcon::Svg(bytes) => {
            row.icon(svg::Handle::from_memory(bytes), None)
        },
        EntryIcon::Letter(name) => row.letter_icon(name),
    }
}

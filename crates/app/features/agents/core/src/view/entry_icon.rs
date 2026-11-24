//! The icon of an agent or a provider: the SVG of its registry entry,
//! tinted, or the first letter of its name on a rounded square when it has
//! none (spec 2026-11-19-agent-plugins, "Links in the interface"); the
//! broken plugin icon in the warn colour, with a tooltip that says so,
//! when the plugin that serves it is broken (issue 17 of spec
//! 2026-11-23-accounts-apart-from-profiles).

use iced::widget::svg;
use iced::{Color, Element};
use otty_app_theme::icons;
use otty_ui_kit::components::letter_icon::letter_icon;
use otty_ui_kit::components::tooltip::tooltip;
use otty_ui_kit::components::tree_row::TreeRow;
use otty_ui_kit::palette::UiPalette;

use super::super::event::AgentsIntent;
use super::super::model::EntryIcon;

/// `icon` as an `edge` pixels square in `color`; the broken plugin icon
/// in the warn colour with its tooltip.
pub(super) fn view<'a>(
    icon: EntryIcon,
    edge: f32,
    color: Color,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let note = icon.note();

    match icon {
        EntryIcon::Svg(bytes) => {
            tinted(svg::Handle::from_memory(bytes), edge, color)
        },
        EntryIcon::Letter(name) => letter_icon(&name, edge, color, palette),
        EntryIcon::Broken(_) => {
            let broken = tinted(
                svg::Handle::from_memory(icons::PLUGIN_BROKEN),
                edge,
                palette.status_warn(),
            );
            tooltip(broken, note.unwrap_or_default(), palette)
        },
    }
}

/// `row` with `icon` before its label; the broken plugin icon in the warn
/// colour. The tooltip of a broken plugin is the caller's, around the row.
pub(super) fn in_row(
    row: TreeRow<AgentsIntent>,
    icon: EntryIcon,
    palette: &UiPalette,
) -> TreeRow<AgentsIntent> {
    match icon {
        EntryIcon::Svg(bytes) => {
            row.icon(svg::Handle::from_memory(bytes), None)
        },
        EntryIcon::Letter(name) => row.letter_icon(name),
        EntryIcon::Broken(_) => row.icon(
            svg::Handle::from_memory(icons::PLUGIN_BROKEN),
            Some(palette.status_warn()),
        ),
    }
}

/// `handle` as an `edge` pixels square in `color`.
fn tinted<'a>(
    handle: svg::Handle,
    edge: f32,
    color: Color,
) -> Element<'a, AgentsIntent> {
    svg(handle)
        .width(edge)
        .height(edge)
        .style(move |_, _| svg::Style { color: Some(color) })
        .into()
}

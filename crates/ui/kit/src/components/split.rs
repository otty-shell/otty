//! `Split`: the style of the 1px `line` between resizable panes.

use iced::widget::pane_grid;
use iced::{Border, Theme};

use crate::palette::UiPalette;
use crate::size;

/// Returns the `PaneGrid` style: the split and the hovered region are drawn
/// in `line`, `size::LINE` wide.
///
/// Pair it with `.spacing(size::LINE)` and `.on_resize(size::SPLIT_GRAB, ..)`.
pub fn style(
    palette: &UiPalette,
) -> impl Fn(&Theme) -> pane_grid::Style + use<> {
    let line = palette.line();

    move |_| pane_grid::Style {
        hovered_region: pane_grid::Highlight {
            background: line.into(),
            border: Border::default(),
        },
        picked_split: pane_grid::Line {
            color: line,
            width: size::LINE,
        },
        hovered_split: pane_grid::Line {
            color: line,
            width: size::LINE,
        },
    }
}

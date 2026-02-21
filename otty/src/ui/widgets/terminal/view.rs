use std::collections::HashMap;

use iced::widget::pane_grid::{self, Highlight, Line, PaneGrid};
use iced::widget::{Stack, container, mouse_area, text};
use iced::{Border, Element, Length, Theme};
use otty_ui_term::TerminalView;

use crate::features::terminal::event::TerminalEvent;
use crate::features::terminal::model::TerminalEntry;

const PANE_GRID_SPACING: f32 = 1.0;
const PANE_RESIZE_GRAB: f32 = 12.0;
const PANE_SEPARATOR_ALPHA: f32 = 0.25;
const PANE_BORDER_WIDTH: f32 = 1.0;

/// Props for rendering a terminal tab.
#[derive(Clone, Copy)]
pub(crate) struct Props<'a> {
    pub(crate) panes: &'a pane_grid::State<u64>,
    pub(crate) terminals: &'a HashMap<u64, TerminalEntry>,
    pub(crate) focus: Option<pane_grid::Pane>,
}

pub(crate) fn view<'a>(props: Props<'a>) -> Element<'a, TerminalEvent> {
    let focus = props.focus;
    let terminals = props.terminals;

    let pane_grid = PaneGrid::new(props.panes, move |pane, terminal_id, _| {
        let is_focused = focus == Some(pane);
        let content =
            view_single_pane(pane, *terminal_id, terminals, is_focused);

        pane_grid::Content::new(content)
    })
    .width(Length::Fill)
    .height(Length::Fill)
    .spacing(PANE_GRID_SPACING)
    .style(|theme: &Theme| {
        let palette = theme.extended_palette();
        let mut separator = palette.background.weak.text;
        separator.a = PANE_SEPARATOR_ALPHA;

        pane_grid::Style {
            hovered_region: Highlight {
                background: separator.into(),
                border: Border::default(),
            },
            picked_split: Line {
                color: separator,
                width: 1.0,
            },
            hovered_split: Line {
                color: separator,
                width: 1.0,
            },
        }
    })
    .on_resize(PANE_RESIZE_GRAB, |event| TerminalEvent::PaneResized {
        event,
    });

    let pane_grid = container(pane_grid)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|theme: &Theme| {
            let palette = theme.extended_palette();
            let mut separator = palette.background.weak.text;
            separator.a = PANE_SEPARATOR_ALPHA;

            iced::widget::container::Style {
                background: Some(separator.into()),
                ..Default::default()
            }
        })
        .into();

    let stack_widget = Stack::with_children(vec![pane_grid])
        .width(Length::Fill)
        .height(Length::Fill);

    mouse_area(stack_widget)
        .on_move(|position| TerminalEvent::PaneGridCursorMoved { position })
        .into()
}

fn view_single_pane<'a>(
    pane: pane_grid::Pane,
    terminal_id: u64,
    terminals: &'a HashMap<u64, TerminalEntry>,
    is_focused: bool,
) -> Element<'a, TerminalEvent> {
    let Some(terminal_entry) = terminals.get(&terminal_id) else {
        log::warn!("terminal pane missing widget: terminal_id={terminal_id}");
        return container(text("Terminal unavailable"))
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
    };

    let focus_event = TerminalEvent::PaneClicked { pane };

    let terminal_view = TerminalView::show(&terminal_entry.terminal)
        .map(TerminalEvent::ProxyToInternalWidget);
    let terminal_area = mouse_area(terminal_view)
        .on_press(focus_event)
        .on_right_press(TerminalEvent::OpenContextMenu { pane, terminal_id })
        .into();

    let mut stack_widget = Stack::with_children(vec![terminal_area]);
    stack_widget = stack_widget.width(Length::Fill).height(Length::Fill);

    container(stack_widget)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |theme: &Theme| {
            let palette = theme.extended_palette();
            let border_color = if is_focused {
                palette.primary.strong.color
            } else {
                palette.background.strong.color
            };

            iced::widget::container::Style {
                border: iced::Border {
                    width: PANE_BORDER_WIDTH,
                    color: border_color,
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .into()
}

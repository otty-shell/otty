use std::collections::HashMap;
use std::process::ExitStatus;

use iced::widget::pane_grid::{self, Highlight, Line, PaneGrid};
use iced::widget::{Stack, container, mouse_area, text};
use iced::{Border, Element, Length, Theme, alignment};
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, spacing, typography};
use otty_ui_term::TerminalView;

use super::super::event::TerminalWorkspaceIntent;
use super::super::model::TerminalTabViewModel;
use super::super::types::TerminalEntry;
use crate::i18n::{self, Key};

/// Width of the 1px `line` between panes. The design system has no size
/// token for dividers.
const DIVIDER_WIDTH: f32 = 1.0;
/// Width of the invisible strip around a divider that starts a resize.
const PANE_RESIZE_GRAB: f32 = 12.0;
/// Width of the pane border that marks the focused pane. No size token.
const PANE_BORDER_WIDTH: f32 = 1.0;

/// Render the pane grid for a terminal tab.
pub(crate) fn view<'a>(
    vm: TerminalTabViewModel<'a>,
    palette: &UiPalette,
) -> Element<'a, TerminalWorkspaceIntent> {
    let tab_id = vm.tab_id;
    let focus = vm.focus;
    let terminals = vm.terminals;
    let line = palette.line();

    let grid = PaneGrid::new(vm.panes, move |pane, terminal_id, _| {
        let is_focused = focus == Some(pane);
        let content = view_single_pane(
            tab_id,
            pane,
            *terminal_id,
            terminals,
            is_focused,
            palette,
        );

        pane_grid::Content::new(content)
    })
    .width(Length::Fill)
    .height(Length::Fill)
    .spacing(DIVIDER_WIDTH)
    .style(move |_: &Theme| pane_grid::Style {
        hovered_region: Highlight {
            background: line.into(),
            border: Border::default(),
        },
        picked_split: Line {
            color: line,
            width: DIVIDER_WIDTH,
        },
        hovered_split: Line {
            color: line,
            width: DIVIDER_WIDTH,
        },
    })
    .on_resize(PANE_RESIZE_GRAB, move |event| {
        TerminalWorkspaceIntent::PaneResized { tab_id, event }
    });

    let grid_container = container(grid)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_: &Theme| iced::widget::container::Style {
            background: Some(line.into()),
            ..Default::default()
        })
        .into();

    let stack_widget = Stack::with_children(vec![grid_container])
        .width(Length::Fill)
        .height(Length::Fill);

    mouse_area(stack_widget)
        .on_move(
            move |position| TerminalWorkspaceIntent::PaneGridCursorMoved {
                tab_id,
                position,
            },
        )
        .into()
}

fn view_single_pane<'a>(
    tab_id: u64,
    pane: pane_grid::Pane,
    terminal_id: u64,
    terminals: &'a HashMap<u64, TerminalEntry>,
    is_focused: bool,
    palette: &UiPalette,
) -> Element<'a, TerminalWorkspaceIntent> {
    let Some(terminal_entry) = terminals.get(&terminal_id) else {
        return container(text(i18n::t(Key::TerminalUnavailable)))
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
    };

    let focus_event = TerminalWorkspaceIntent::PaneClicked { tab_id, pane };

    let terminal_view = TerminalView::show(terminal_entry.terminal())
        .map(TerminalWorkspaceIntent::Widget);
    let terminal_area = mouse_area(terminal_view)
        .on_press(focus_event)
        .on_right_press(TerminalWorkspaceIntent::OpenContextMenu {
            tab_id,
            pane,
            terminal_id,
        })
        .into();

    let mut layers = vec![terminal_area];
    if let Some(exit_status) = terminal_entry.exit_status() {
        layers.push(exit_banner(exit_status, palette));
    }

    let stack_widget = Stack::with_children(layers)
        .width(Length::Fill)
        .height(Length::Fill);

    let canvas = palette.surface_canvas();
    let border_color = if is_focused {
        palette.focus()
    } else {
        palette.line()
    };

    container(stack_widget)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_: &Theme| iced::widget::container::Style {
            background: Some(canvas.into()),
            border: iced::Border {
                width: PANE_BORDER_WIDTH,
                color: border_color,
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}

/// Bottom strip telling the user the process ended and how to close the pane.
fn exit_banner<'a>(
    exit_status: ExitStatus,
    palette: &UiPalette,
) -> Element<'a, TerminalWorkspaceIntent> {
    let message = i18n::process_exited(exit_status.code());
    let background = palette.surface_overlay();
    let text_color = palette.text_body();

    let banner =
        container(text(message).size(typography::BODY.size()).font(fonts::UI))
            .width(Length::Fill)
            .padding(spacing::SPACE_3)
            .style(move |_: &Theme| iced::widget::container::Style {
                background: Some(background.into()),
                text_color: Some(text_color),
                ..Default::default()
            });

    container(banner)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_y(alignment::Vertical::Bottom)
        .into()
}

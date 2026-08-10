use std::collections::HashMap;

use iced::widget::pane_grid::{self, Highlight, Line, PaneGrid};
use iced::widget::{Stack, container, mouse_area, text};
use iced::{Border, Element, Length, Theme, alignment};
use otty_ui_term::{DegradedReason, IntegrationStatus, TerminalView};

use super::super::event::TerminalWorkspaceIntent;
use super::super::model::TerminalTabViewModel;
use super::super::types::TerminalEntry;

const PANE_GRID_SPACING: f32 = 1.0;
const PANE_RESIZE_GRAB: f32 = 12.0;
const PANE_SEPARATOR_ALPHA: f32 = 0.25;
const PANE_BORDER_WIDTH: f32 = 1.0;

/// Render the pane grid for a terminal tab.
pub(crate) fn view<'a>(
    vm: TerminalTabViewModel<'a>,
) -> Element<'a, TerminalWorkspaceIntent> {
    let tab_id = vm.tab_id;
    let focus = vm.focus;
    let terminals = vm.terminals;

    let grid = PaneGrid::new(vm.panes, move |pane, terminal_id, _| {
        let is_focused = focus == Some(pane);
        let content = view_single_pane(
            tab_id,
            pane,
            *terminal_id,
            terminals,
            is_focused,
            // TODO(B2-104): Убрать после завершения реализации V2.
            vm.is_shell,
        );

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
    .on_resize(PANE_RESIZE_GRAB, move |event| {
        TerminalWorkspaceIntent::PaneResized { tab_id, event }
    });

    let grid_container = container(grid)
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
    // TODO(B2-104): Убрать после завершения реализации V2.
    is_shell: bool,
) -> Element<'a, TerminalWorkspaceIntent> {
    let Some(terminal_entry) = terminals.get(&terminal_id) else {
        return container(text("Terminal unavailable"))
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

    let mut stack_widget = Stack::with_children(vec![terminal_area])
        .width(Length::Fill)
        .height(Length::Fill);
    // TODO(B2-104): Убрать после завершения реализации V2.
    if is_shell {
        let integration_status = terminal_entry
            .terminal()
            .snapshot_arc()
            .view()
            .integration_status
            .clone();
        stack_widget =
            stack_widget.push(integration_status_badge(integration_status));
    }

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

// TODO(B2-104): Убрать после завершения реализации V2.
fn integration_status_badge<'a>(
    status: IntegrationStatus,
) -> Element<'a, TerminalWorkspaceIntent> {
    let label = integration_status_label(&status);
    let is_active = matches!(status, IntegrationStatus::Active(_));
    let is_pending = matches!(status, IntegrationStatus::Pending);
    let badge = container(text(label).size(11)).padding([2, 6]).style(
        move |theme: &Theme| {
            let palette = theme.extended_palette();
            let mut background = if is_active {
                palette.success.weak.color
            } else if is_pending {
                palette.warning.weak.color
            } else {
                palette.danger.weak.color
            };
            background.a = 0.9;

            iced::widget::container::Style {
                background: Some(background.into()),
                text_color: Some(if is_active {
                    palette.success.weak.text
                } else if is_pending {
                    palette.warning.weak.text
                } else {
                    palette.danger.weak.text
                }),
                border: Border::default().rounded(4),
                ..Default::default()
            }
        },
    );

    container(badge)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(6)
        .align_x(alignment::Horizontal::Right)
        .align_y(alignment::Vertical::Top)
        .into()
}

// TODO(B2-104): Убрать после завершения реализации V2.
fn integration_status_label(status: &IntegrationStatus) -> String {
    match status {
        IntegrationStatus::Pending => {
            String::from("Blocks integration pending")
        },
        IntegrationStatus::Active(version) => {
            format!("Blocks v{version} active")
        },
        IntegrationStatus::Degraded(DegradedReason::SequenceGap {
            expected,
            received,
        }) => format!("Blocks degraded: event gap {expected}→{received}"),
        IntegrationStatus::Degraded(DegradedReason::MissingShellHello) => {
            String::from("Blocks degraded: handshake missing")
        },
        IntegrationStatus::Degraded(DegradedReason::IntegrationError(code)) => {
            format!("Blocks degraded: {code}")
        },
        IntegrationStatus::UnsupportedVersion(version) => {
            format!("Blocks protocol v{version} unsupported")
        },
        IntegrationStatus::Unsupported(shell) => {
            format!("Blocks unsupported for {shell}")
        },
    }
}

#[cfg(test)]
mod tests {
    use otty_ui_term::{DegradedReason, IntegrationStatus, ProtocolSequence};

    use super::integration_status_label;

    // TODO(B2-104): Убрать после завершения реализации V2.
    #[test]
    fn integration_badge_explains_active_and_degraded_states() {
        assert_eq!(
            integration_status_label(&IntegrationStatus::Active(2)),
            "Blocks v2 active"
        );
        assert_eq!(
            integration_status_label(&IntegrationStatus::Degraded(
                DegradedReason::SequenceGap {
                    expected: ProtocolSequence::new(4),
                    received: ProtocolSequence::new(6),
                }
            )),
            "Blocks degraded: event gap 4→6"
        );
    }
}

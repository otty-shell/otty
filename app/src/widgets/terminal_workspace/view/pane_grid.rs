use std::collections::HashMap;
use std::process::ExitStatus;

use iced::widget::pane_grid::{self, PaneGrid};
use iced::widget::{Stack, container, mouse_area, text};
use iced::{Element, Length, Theme, alignment};
use otty_ui_kit::components::chip::Chip;
use otty_ui_kit::components::pane_header::PaneHeader;
use otty_ui_kit::components::split::{self, ResizableGrid};
use otty_ui_kit::components::target_dot::TargetKind;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, size, spacing, typography};
use otty_ui_term::TerminalView;

use super::super::event::TerminalWorkspaceIntent;
use super::super::model::PaneGridViewModel;
use super::super::types::TerminalEntry;
use crate::i18n::{self, Key};

/// Smallest width or height of a terminal pane while a split is dragged.
const MIN_PANE_SIZE: f32 = 50.0;

/// Render the pane grid for a terminal tab.
pub(crate) fn view<'a>(
    vm: PaneGridViewModel<'a>,
    palette: &UiPalette,
) -> Element<'a, TerminalWorkspaceIntent> {
    let tab_id = vm.tab_id;
    let focus = vm.focus;
    let terminals = vm.terminals;
    let machine = vm.machine;
    let machine_target = vm.machine_target;
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

        let pane_content = pane_grid::Content::new(content);
        match terminals.get(terminal_id) {
            Some(entry) => {
                let header = view_pane_header(
                    tab_id,
                    pane,
                    entry,
                    machine,
                    machine_target,
                    is_focused,
                    palette,
                );
                pane_content.title_bar(pane_grid::TitleBar::new(header))
            },
            None => pane_content,
        }
    })
    .width(Length::Fill)
    .height(Length::Fill)
    .spacing(size::LINE)
    .min_size(MIN_PANE_SIZE)
    .style(split::style(palette));
    let grid =
        ResizableGrid::new(grid, vm.panes, MIN_PANE_SIZE, palette).on_resize(
            move |event| TerminalWorkspaceIntent::PaneResized { tab_id, event },
        );

    // The container is painted `line` so the 1px gaps between panes show as rules.
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

/// Bar above a pane with its program, folder and machine. Pressing it
/// focuses the pane, like pressing the terminal.
fn view_pane_header<'a>(
    tab_id: u64,
    pane: pane_grid::Pane,
    entry: &TerminalEntry,
    machine: &str,
    machine_target: TargetKind,
    is_focused: bool,
    palette: &UiPalette,
) -> Element<'a, TerminalWorkspaceIntent> {
    let label = entry.label();
    let chip = Chip::new(machine, palette).target(machine_target);

    let mut header = PaneHeader::new(label.program(), palette)
        .focused(is_focused)
        .trailing(chip);
    if let Some(folder) = label.folder_name() {
        header = header.folder(folder);
    }

    mouse_area(Element::from(header))
        .on_press(TerminalWorkspaceIntent::PaneClicked { tab_id, pane })
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
                width: size::LINE,
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

#[cfg(test)]
mod tests {
    use iced::widget::pane_grid::{Axis, Configuration};
    use iced::{Event, Point, Size, mouse};
    use iced_test::Simulator;

    use super::*;

    const WINDOW: Size = Size::new(400.0, 300.0);
    /// Where the line of a split at the middle of [`WINDOW`] starts.
    const LINE: f32 = 200.0;

    fn split_panes() -> pane_grid::State<u64> {
        pane_grid::State::with_configuration(Configuration::Split {
            axis: Axis::Vertical,
            ratio: 0.5,
            a: Box::new(Configuration::Pane(1)),
            b: Box::new(Configuration::Pane(2)),
        })
    }

    /// Presses at `x` in a grid of two panes, moves and releases; returns
    /// whether a resize of pane splits was sent.
    fn resized_by_a_press_at(x: f32) -> bool {
        let panes = split_panes();
        let terminals = HashMap::new();
        let palette = UiPalette::otty_dark();
        let grid = view(
            PaneGridViewModel {
                tab_id: 7,
                panes: &panes,
                terminals: &terminals,
                focus: None,
                machine: "host",
                machine_target: TargetKind::Local,
            },
            &palette,
        );
        let mut ui =
            Simulator::with_size(iced::Settings::default(), WINDOW, grid);

        for (index, x) in [x, 300.0].into_iter().enumerate() {
            let position = Point::new(x, 150.0);
            ui.point_at(position);
            let _ = ui.simulate([Event::Mouse(mouse::Event::CursorMoved {
                position,
            })]);
            if index == 0 {
                let _ = ui.simulate([Event::Mouse(
                    mouse::Event::ButtonPressed(mouse::Button::Left),
                )]);
            }
        }
        let _ = ui.simulate([Event::Mouse(mouse::Event::ButtonReleased(
            mouse::Button::Left,
        ))]);

        ui.into_messages().any(|intent| {
            matches!(
                intent,
                TerminalWorkspaceIntent::PaneResized { tab_id: 7, .. }
            )
        })
    }

    #[test]
    fn a_press_on_a_pane_split_resizes_the_panes() {
        assert!(resized_by_a_press_at(LINE + 0.5));
        assert!(resized_by_a_press_at(LINE + 3.0));
    }

    #[test]
    fn a_press_left_of_the_line_stays_with_the_left_terminal() {
        assert!(!resized_by_a_press_at(LINE - 2.0));
    }

    #[test]
    fn a_press_away_from_the_line_does_not_resize() {
        assert!(!resized_by_a_press_at(LINE + 20.0));
        assert!(!resized_by_a_press_at(LINE - 20.0));
    }
}

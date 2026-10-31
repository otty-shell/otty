use iced::widget::{Id, container, mouse_area, pane_grid, text_input};
use iced::{Background, Element, Length, Point, Size, alignment};
use otty_ui_kit::components::menu::{self, Menu, MenuItem};
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{size, spacing};

use crate::geometry::anchor_position;
use crate::i18n::{self, Key};
use crate::shortcuts::AppShortcut;
use crate::theme::ThemeProps;
use crate::widgets::terminal_workspace::event::TerminalWorkspaceIntent;

/// Size of the invisible text field that keeps keyboard focus while the menu
/// is open. It is not a design value, so it has no token.
const FOCUS_TRAP_SIZE: f32 = 1.0;

/// Height of a menu separator: 1px line plus 4px margin above and below, as
/// drawn by the kit menu.
const SEPARATOR_HEIGHT: f32 = 9.0;

/// Props for rendering a pane context menu.
#[derive(Debug, Clone)]
pub(crate) struct PaneContextMenuProps<'a> {
    pub(crate) tab_id: u64,
    pub(crate) pane: pane_grid::Pane,
    pub(crate) cursor: Point,
    pub(crate) grid_size: Size,
    pub(crate) terminal_id: u64,
    pub(crate) focus_target: Id,
    pub(crate) has_block_selection: bool,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the context menu overlay for a terminal pane.
pub(crate) fn view<'a>(
    props: PaneContextMenuProps<'a>,
) -> Element<'a, TerminalWorkspaceIntent> {
    let tab_id = props.tab_id;
    let terminal_id = props.terminal_id;
    let pane = props.pane;
    let mut menu = Menu::new(props.theme.theme.ui())
        .item(MenuItem::new(i18n::t(Key::CopySelection)).on_press(
            TerminalWorkspaceIntent::CopySelection {
                tab_id,
                terminal_id,
            },
        ))
        .item(MenuItem::new(i18n::t(Key::Paste)).on_press(
            TerminalWorkspaceIntent::PasteIntoPrompt {
                tab_id,
                terminal_id,
            },
        ))
        .separator();

    if props.has_block_selection {
        menu = menu
            .item(MenuItem::new(i18n::t(Key::CopyContent)).on_press(
                TerminalWorkspaceIntent::CopySelectedBlockContent {
                    tab_id,
                    terminal_id,
                },
            ))
            .item(MenuItem::new(i18n::t(Key::CopyPrompt)).on_press(
                TerminalWorkspaceIntent::CopySelectedBlockPrompt {
                    tab_id,
                    terminal_id,
                },
            ))
            .item(MenuItem::new(i18n::t(Key::CopyCommand)).on_press(
                TerminalWorkspaceIntent::CopySelectedBlockCommand {
                    tab_id,
                    terminal_id,
                },
            ))
            .separator();
    }

    menu = menu
        .item(
            MenuItem::new(i18n::t(Key::SplitRight))
                .shortcut(AppShortcut::SplitPane.label())
                .on_press(TerminalWorkspaceIntent::SplitPane {
                    tab_id,
                    pane,
                    axis: pane_grid::Axis::Vertical,
                }),
        )
        .item(MenuItem::new(i18n::t(Key::SplitDown)).on_press(
            TerminalWorkspaceIntent::SplitPane {
                tab_id,
                pane,
                axis: pane_grid::Axis::Horizontal,
            },
        ))
        .separator()
        .item(
            MenuItem::new(i18n::t(Key::ClosePane))
                .on_press(TerminalWorkspaceIntent::ClosePane { tab_id, pane }),
        );

    let anchor = anchor_position(
        props.cursor,
        props.grid_size,
        menu::MENU_WIDTH,
        pane_menu_height(props.has_block_selection),
        spacing::SPACE_2,
    );

    let padding = iced::Padding {
        top: anchor.y,
        left: anchor.x,
        ..iced::Padding::ZERO
    };

    let positioned_menu = container(Element::from(menu))
        .padding(padding)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(alignment::Horizontal::Left)
        .align_y(alignment::Vertical::Top);

    let dismiss_layer = mouse_area(
        container(iced::widget::text(""))
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .on_press(TerminalWorkspaceIntent::CloseContextMenu {
        tab_id: props.tab_id,
    })
    .on_right_press(TerminalWorkspaceIntent::CloseContextMenu {
        tab_id: props.tab_id,
    })
    .on_move(move |position| {
        TerminalWorkspaceIntent::PaneGridCursorMoved {
            tab_id: props.tab_id,
            position,
        }
    });

    iced::widget::stack!(
        dismiss_layer,
        positioned_menu,
        focus_trap(
            props.tab_id,
            props.focus_target.clone(),
            props.theme.theme.ui()
        )
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

/// Total height of the pane menu, used to keep it inside the pane grid.
fn pane_menu_height(has_block_selection: bool) -> f32 {
    let (rows, separators) = if has_block_selection { (8, 3) } else { (5, 2) };

    2.0 * menu::MENU_PADDING
        + size::ROW * rows as f32
        + SEPARATOR_HEIGHT * separators as f32
}

fn focus_trap(
    tab_id: u64,
    id: Id,
    palette: &UiPalette,
) -> Element<'static, TerminalWorkspaceIntent> {
    // Every part of the field has the canvas color, so it stays invisible.
    let color = palette.surface_canvas();

    text_input("", "")
        .on_input(move |_| TerminalWorkspaceIntent::ContextMenuInput { tab_id })
        .padding(0)
        .size(FOCUS_TRAP_SIZE)
        .width(Length::Fixed(FOCUS_TRAP_SIZE))
        .id(id)
        .style(move |_: &iced::Theme, _status| {
            iced::widget::text_input::Style {
                background: Background::Color(color),
                border: iced::Border {
                    width: 0.0,
                    ..Default::default()
                },
                icon: color,
                placeholder: color,
                value: color,
                selection: color,
            }
        })
        .into()
}

#[cfg(test)]
mod tests {
    use otty_ui_kit::components::menu;
    use otty_ui_kit::size;

    use super::{SEPARATOR_HEIGHT, pane_menu_height};

    #[test]
    fn given_no_block_selection_when_height_computed_then_five_rows_and_two_separators()
     {
        let expected =
            2.0 * menu::MENU_PADDING + 5.0 * size::ROW + 2.0 * SEPARATOR_HEIGHT;

        assert_eq!(pane_menu_height(false), expected);
    }

    #[test]
    fn given_block_selection_when_height_computed_then_three_rows_and_one_separator_are_added()
     {
        let growth = pane_menu_height(true) - pane_menu_height(false);

        assert_eq!(growth, 3.0 * size::ROW + SEPARATOR_HEIGHT);
    }
}

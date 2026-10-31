use iced::widget::{Id, container, mouse_area, pane_grid, text_input};
use iced::{Background, Element, Length, Point, Size, alignment};
use otty_ui_kit::components::menu::{self, Menu, MenuItem};

use crate::geometry::{MENU_MARGIN, anchor_position, menu_height_for_items};
use crate::i18n::{self, Key};
use crate::theme::ThemeProps;
use crate::widgets::terminal_workspace::event::TerminalWorkspaceIntent;
use crate::widgets::terminal_workspace::shortcuts::split_vertically_shortcut_label;

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
        ));
    let mut item_count = 2;

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
            ));
        item_count += 3;
    }

    menu = menu
        .item(MenuItem::new(i18n::t(Key::SplitHorizontally)).on_press(
            TerminalWorkspaceIntent::SplitPane {
                tab_id,
                pane,
                axis: pane_grid::Axis::Horizontal,
            },
        ))
        .item(
            MenuItem::new(i18n::t(Key::SplitVertically))
                .shortcut(split_vertically_shortcut_label())
                .on_press(TerminalWorkspaceIntent::SplitPane {
                    tab_id,
                    pane,
                    axis: pane_grid::Axis::Vertical,
                }),
        )
        .item(
            MenuItem::new(i18n::t(Key::ClosePane))
                .on_press(TerminalWorkspaceIntent::ClosePane { tab_id, pane }),
        );
    item_count += 3;

    let anchor = anchor_position(
        props.cursor,
        props.grid_size,
        menu::MENU_WIDTH,
        menu_height_for_items(item_count),
        MENU_MARGIN,
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
        focus_trap(props.tab_id, props.focus_target.clone())
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn focus_trap(
    tab_id: u64,
    id: Id,
) -> Element<'static, TerminalWorkspaceIntent> {
    text_input("", "")
        .on_input(move |_| TerminalWorkspaceIntent::ContextMenuInput { tab_id })
        .padding(0)
        .size(1)
        .width(Length::Fixed(1.0))
        .id(id)
        .style(|theme: &iced::Theme, _status| {
            let color = theme.extended_palette().background.base.color;
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

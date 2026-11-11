use iced::widget::{container, mouse_area, pane_grid};
use iced::{Element, Length, Point, Size, alignment};
use otty_app_domain::shortcuts::AppShortcut;
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::menu::{self, Menu, MenuItem};
use otty_ui_kit::geometry::anchor_position;
use otty_ui_kit::{size, spacing};

use crate::event::TerminalIntent;

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
    pub(crate) has_block_selection: bool,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the context menu overlay for a terminal pane.
pub(crate) fn view<'a>(
    props: PaneContextMenuProps<'a>,
) -> Element<'a, TerminalIntent> {
    let tab_id = props.tab_id;
    let terminal_id = props.terminal_id;
    let pane = props.pane;
    let mut menu = Menu::new(props.theme.theme.ui())
        .item(MenuItem::new(i18n::t(Key::CopySelection)).on_press(
            TerminalIntent::CopySelection {
                tab_id,
                terminal_id,
            },
        ))
        .item(MenuItem::new(i18n::t(Key::Paste)).on_press(
            TerminalIntent::PasteIntoPrompt {
                tab_id,
                terminal_id,
            },
        ))
        .separator();

    if props.has_block_selection {
        menu = menu
            .item(MenuItem::new(i18n::t(Key::CopyContent)).on_press(
                TerminalIntent::CopySelectedBlockContent {
                    tab_id,
                    terminal_id,
                },
            ))
            .item(MenuItem::new(i18n::t(Key::CopyCommand)).on_press(
                TerminalIntent::CopySelectedBlockCommand {
                    tab_id,
                    terminal_id,
                },
            ))
            .separator();
    }

    menu = menu
        .item(
            MenuItem::new(i18n::t(Key::SplitRight))
                .shortcut(AppShortcut::SplitRight.label())
                .on_press(TerminalIntent::SplitPane {
                    tab_id,
                    pane,
                    axis: pane_grid::Axis::Vertical,
                }),
        )
        .item(
            MenuItem::new(i18n::t(Key::SplitDown))
                .shortcut(AppShortcut::SplitDown.label())
                .on_press(TerminalIntent::SplitPane {
                    tab_id,
                    pane,
                    axis: pane_grid::Axis::Horizontal,
                }),
        )
        .separator()
        .item(
            MenuItem::new(i18n::t(Key::ClosePane))
                .on_press(TerminalIntent::ClosePane { tab_id, pane }),
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
    .on_press(TerminalIntent::CloseContextMenu {
        tab_id: props.tab_id,
    })
    .on_right_press(TerminalIntent::CloseContextMenu {
        tab_id: props.tab_id,
    })
    .on_middle_press(TerminalIntent::CloseContextMenu {
        tab_id: props.tab_id,
    })
    .on_move(move |position| TerminalIntent::PaneGridCursorMoved {
        tab_id: props.tab_id,
        position,
    });

    iced::widget::stack!(dismiss_layer, positioned_menu)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Total height of the pane menu, used to keep it inside the pane grid.
fn pane_menu_height(has_block_selection: bool) -> f32 {
    let (rows, separators) = if has_block_selection { (7, 3) } else { (5, 2) };

    2.0 * menu::MENU_PADDING
        + size::ROW * rows as f32
        + SEPARATOR_HEIGHT * separators as f32
}

#[cfg(test)]
mod tests {
    use iced::widget::pane_grid;
    use iced::{Point, Size};
    use iced_test::simulator;
    use otty_app_i18n::{self as i18n, Key};
    use otty_app_testing::mouse::middle_click;
    use otty_app_theme::theme::{ThemeManager, ThemeProps};
    use otty_ui_kit::components::menu;
    use otty_ui_kit::size;

    use super::{
        PaneContextMenuProps, SEPARATOR_HEIGHT, pane_menu_height, view,
    };
    use crate::event::TerminalIntent;

    #[test]
    fn given_no_block_selection_when_height_computed_then_five_rows_and_two_separators()
     {
        let expected =
            2.0 * menu::MENU_PADDING + 5.0 * size::ROW + 2.0 * SEPARATOR_HEIGHT;

        assert_eq!(pane_menu_height(false), expected);
    }

    #[test]
    fn given_block_selection_when_height_computed_then_two_rows_and_one_separator_are_added()
     {
        let growth = pane_menu_height(true) - pane_menu_height(false);

        assert_eq!(growth, 2.0 * size::ROW + SEPARATOR_HEIGHT);
    }

    /// The block items copy the output and the command; the prompt is not
    /// offered.
    #[test]
    fn a_selected_block_offers_its_content_and_command_but_not_its_prompt() {
        let (_, pane) = pane_grid::State::new(());
        let themes = ThemeManager::new();
        let mut ui = simulator(view(PaneContextMenuProps {
            tab_id: 7,
            pane,
            cursor: Point::new(40.0, 40.0),
            grid_size: Size::new(1024.0, 768.0),
            terminal_id: 3,
            has_block_selection: true,
            theme: ThemeProps::new(themes.current()),
        }));

        assert!(ui.find(i18n::t(Key::CopyContent)).is_ok());
        assert!(ui.find(i18n::t(Key::CopyCommand)).is_ok());
        assert!(ui.find("Copy prompt").is_err());
    }

    /// A middle press away from the menu closes it, as a left press does,
    /// instead of reaching the terminal under it.
    #[test]
    fn a_middle_press_outside_the_menu_closes_it() {
        let (_, pane) = pane_grid::State::new(());
        let themes = ThemeManager::new();
        let mut ui = simulator(view(PaneContextMenuProps {
            tab_id: 7,
            pane,
            cursor: Point::new(40.0, 40.0),
            grid_size: Size::new(1024.0, 768.0),
            terminal_id: 3,
            has_block_selection: false,
            theme: ThemeProps::new(themes.current()),
        }));

        ui.point_at(Point::new(900.0, 700.0));
        let _ = ui.simulate(middle_click());

        // The layer also reports where the pointer is.
        let intents: Vec<TerminalIntent> = ui
            .into_messages()
            .filter(|intent| {
                !matches!(intent, TerminalIntent::PaneGridCursorMoved { .. })
            })
            .collect();
        assert!(matches!(
            intents.as_slice(),
            [TerminalIntent::CloseContextMenu { tab_id: 7 }]
        ));
    }
}

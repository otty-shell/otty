use iced::widget::{mouse_area, pane_grid};
use iced::{Element, Point, Size};
use otty_app_domain::shortcuts::AppShortcut;
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::context_menu::context_menu;
use otty_ui_kit::components::menu::{Menu, MenuItem};

use crate::event::TerminalIntent;

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

/// Render the context menu of a terminal pane at the pointer, kept inside
/// the pane grid. A press away from it closes it.
pub(crate) fn view<'a>(
    props: PaneContextMenuProps<'a>,
) -> Element<'a, TerminalIntent> {
    let tab_id = props.tab_id;
    let cursor = props.cursor;
    let grid_size = props.grid_size;

    let layer = context_menu(
        menu(props),
        cursor,
        grid_size,
        TerminalIntent::CloseContextMenu { tab_id },
    );

    // The layer covers the pane grid, so it reports where the pointer is
    // in place of the grid: the next menu opens at the right spot.
    mouse_area(layer)
        .on_move(move |position| TerminalIntent::PaneGridCursorMoved {
            tab_id,
            position,
        })
        .into()
}

/// The menu of a pane: copy and paste, the content and the command of the
/// selected block when there is one, the splits, and closing the pane.
fn menu(props: PaneContextMenuProps<'_>) -> Menu<TerminalIntent> {
    let tab_id = props.tab_id;
    let terminal_id = props.terminal_id;
    let pane = props.pane;

    let menu = Menu::new(props.theme.theme.ui())
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

    let menu = if props.has_block_selection {
        menu.item(MenuItem::new(i18n::t(Key::CopyContent)).on_press(
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
        .separator()
    } else {
        menu
    };

    menu.item(
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
    )
}

#[cfg(test)]
mod tests {
    use iced::widget::pane_grid;
    use iced::{Point, Size};
    use iced_test::simulator;
    use otty_app_i18n::{self as i18n, Key};
    use otty_app_testing::mouse::middle_click;
    use otty_app_theme::theme::{ThemeManager, ThemeProps};

    use super::{PaneContextMenuProps, view};
    use crate::event::TerminalIntent;

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

    /// The open menu covers the pane grid, so it reports the pointer in
    /// place of the grid: the next menu opens where the pointer is.
    #[test]
    fn the_open_menu_reports_where_the_pointer_is() {
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
        let pointer = Point::new(900.0, 700.0);

        ui.point_at(pointer);
        let _ = ui.simulate(middle_click());

        let is_reported = ui.into_messages().any(|intent| {
            matches!(
                intent,
                TerminalIntent::PaneGridCursorMoved { tab_id: 7, position }
                    if position == pointer
            )
        });
        assert!(is_reported);
    }
}

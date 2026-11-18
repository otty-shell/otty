use iced::widget::{mouse_area, pane_grid};
use iced::{Element, Point, Size};
use otty_app_domain::shortcuts::AppShortcut;
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::context_menu::{MenuPlace, context_menu};
use otty_ui_kit::components::menu::{Menu, MenuItem};
use otty_ui_term::BlockCommand;

use crate::event::TerminalIntent;
use crate::model::{BlockMenu, FoldAction};

/// Props for rendering a pane context menu.
#[derive(Debug, Clone)]
pub(crate) struct PaneContextMenuProps<'a> {
    pub(crate) tab_id: u64,
    pub(crate) pane: pane_grid::Pane,
    pub(crate) cursor: Point,
    pub(crate) grid_size: Size,
    pub(crate) terminal_id: u64,
    /// What the menu offers for the block it acts on, the selected block
    /// of the terminal; `None` when no block of the terminal is selected.
    pub(crate) block: Option<BlockMenu>,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the context menu of a terminal pane at the pointer, kept inside
/// the pane grid. A press away from it closes it.
pub(crate) fn view<'a>(
    props: PaneContextMenuProps<'a>,
) -> Element<'a, TerminalIntent> {
    let tab_id = props.tab_id;
    let place = MenuPlace::new(props.cursor, props.grid_size);

    let layer = context_menu(
        menu(props),
        place,
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

/// The menu of a pane: copy and paste, the items of the selected block when
/// there is one (fold, run again, copy its content and its command), the
/// splits, and closing the pane.
fn menu(props: PaneContextMenuProps<'_>) -> Menu<TerminalIntent> {
    let tab_id = props.tab_id;
    let terminal_id = props.terminal_id;
    let pane = props.pane;

    let menu = Menu::new(props.theme.theme.ui())
        .item(MenuItem::new(i18n::t(Key::CopySelection)).on_press(
            TerminalIntent::RunBlockCommand {
                tab_id,
                terminal_id,
                command: BlockCommand::CopySelection,
            },
        ))
        .item(MenuItem::new(i18n::t(Key::Paste)).on_press(
            TerminalIntent::RunBlockCommand {
                tab_id,
                terminal_id,
                command: BlockCommand::PasteClipboard,
            },
        ))
        .separator();

    let menu = match props.block {
        Some(block) => block_items(menu, block, tab_id, terminal_id),
        None => menu,
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

/// Add the items of the block the menu acts on: `Collapse` or `Expand`
/// when it can be folded, `Run again` when it can run again, and copying
/// its content and its command. Each item sends the command ready for that
/// block, named by its key.
fn block_items(
    menu: Menu<TerminalIntent>,
    block: BlockMenu,
    tab_id: u64,
    terminal_id: u64,
) -> Menu<TerminalIntent> {
    let block_key = block.block_key();
    let run = |command| TerminalIntent::RunBlockCommand {
        tab_id,
        terminal_id,
        command,
    };

    let fold = block.fold().map(|action| {
        let label = match action {
            FoldAction::Collapse => Key::Collapse,
            FoldAction::Expand => Key::Expand,
        };

        MenuItem::new(i18n::t(label))
            .on_press(run(BlockCommand::ToggleFold(block_key)))
    });
    let run_again = block.can_run_again().then(|| {
        MenuItem::new(i18n::t(Key::RunAgain))
            .on_press(run(BlockCommand::RunAgain(block_key)))
    });

    let menu = match fold {
        Some(item) => menu.item(item),
        None => menu,
    };
    let menu = match run_again {
        Some(item) => menu.item(item),
        None => menu,
    };

    menu.item(
        MenuItem::new(i18n::t(Key::CopyContent))
            .on_press(run(BlockCommand::CopyContent(block_key))),
    )
    .item(
        MenuItem::new(i18n::t(Key::CopyCommand))
            .on_press(run(BlockCommand::CopyCommand(block_key))),
    )
    .separator()
}

#[cfg(test)]
mod tests {
    use iced::widget::pane_grid;
    use iced::{Point, Size};
    use iced_test::simulator;
    use otty_app_i18n::{self as i18n, Key};
    use otty_app_testing::mouse::middle_click;
    use otty_app_theme::theme::{ThemeManager, ThemeProps};
    use otty_ui_term::{BlockCommand, BlockKey};

    use super::{PaneContextMenuProps, view};
    use crate::event::TerminalIntent;
    use crate::model::{BlockMenu, FoldAction};

    /// The key the selected block has in these tests.
    const KEY: BlockKey = BlockKey::new(5);

    /// The intents a press on the item `label` of the menu of a pane sends,
    /// when the menu acts on a block that offers `block`.
    fn pressed(block: BlockMenu, label: &str) -> Vec<TerminalIntent> {
        let (_, pane) = pane_grid::State::new(());
        let themes = ThemeManager::new();
        let mut ui = simulator(view(PaneContextMenuProps {
            tab_id: 7,
            pane,
            cursor: Point::new(40.0, 40.0),
            grid_size: Size::new(1024.0, 768.0),
            terminal_id: 3,
            block: Some(block),
            theme: ThemeProps::new(themes.current()),
        }));

        let _ = ui.click(label).expect("the menu has the item");

        ui.into_messages()
            .filter(|intent| {
                !matches!(intent, TerminalIntent::PaneGridCursorMoved { .. })
            })
            .collect()
    }

    /// Whether the menu on a block that offers `block` has the item
    /// `label`.
    fn has_item(block: BlockMenu, label: &str) -> bool {
        let (_, pane) = pane_grid::State::new(());
        let themes = ThemeManager::new();
        let mut ui = simulator(view(PaneContextMenuProps {
            tab_id: 7,
            pane,
            cursor: Point::new(40.0, 40.0),
            grid_size: Size::new(1024.0, 768.0),
            terminal_id: 3,
            block: Some(block),
            theme: ThemeProps::new(themes.current()),
        }));

        ui.find(label).is_ok()
    }

    /// The block items copy the output and the command; the prompt is not
    /// offered.
    #[test]
    fn a_selected_block_offers_its_content_and_command_but_not_its_prompt() {
        let block = BlockMenu::new(KEY, None, false);

        assert!(has_item(block, i18n::t(Key::CopyContent)));
        assert!(has_item(block, i18n::t(Key::CopyCommand)));
        assert!(!has_item(block, "Copy prompt"));
    }

    #[test]
    fn an_open_block_that_can_be_folded_offers_collapse_and_a_folded_one_expand()
     {
        let open = BlockMenu::new(KEY, Some(FoldAction::Collapse), true);
        let folded = BlockMenu::new(KEY, Some(FoldAction::Expand), true);

        assert!(has_item(open, i18n::t(Key::Collapse)));
        assert!(!has_item(open, i18n::t(Key::Expand)));
        assert!(has_item(folded, i18n::t(Key::Expand)));
        assert!(!has_item(folded, i18n::t(Key::Collapse)));
    }

    #[test]
    fn a_block_that_can_not_be_folded_offers_neither_collapse_nor_expand() {
        let running = BlockMenu::new(KEY, None, false);

        assert!(!has_item(running, i18n::t(Key::Collapse)));
        assert!(!has_item(running, i18n::t(Key::Expand)));
    }

    #[test]
    fn run_again_is_offered_only_when_the_block_can_run_again() {
        let at_prompt = BlockMenu::new(KEY, None, true);
        let while_running = BlockMenu::new(KEY, None, false);

        assert!(has_item(at_prompt, i18n::t(Key::RunAgain)));
        assert!(!has_item(while_running, i18n::t(Key::RunAgain)));
    }

    /// Every block item sends the ready command for the block the menu was
    /// built for.
    #[test]
    fn every_block_item_acts_on_the_block_of_the_menu() {
        let block = BlockMenu::new(KEY, Some(FoldAction::Collapse), true);

        let fold = pressed(block, i18n::t(Key::Collapse));
        let run = pressed(block, i18n::t(Key::RunAgain));
        let content = pressed(block, i18n::t(Key::CopyContent));
        let command = pressed(block, i18n::t(Key::CopyCommand));

        assert!(
            matches!(sent_command(&fold), Some(BlockCommand::ToggleFold(KEY))),
            "got {fold:?}"
        );
        assert!(
            matches!(sent_command(&run), Some(BlockCommand::RunAgain(KEY))),
            "got {run:?}"
        );
        assert!(
            matches!(
                sent_command(&content),
                Some(BlockCommand::CopyContent(KEY))
            ),
            "got {content:?}"
        );
        assert!(
            matches!(
                sent_command(&command),
                Some(BlockCommand::CopyCommand(KEY))
            ),
            "got {command:?}"
        );
    }

    /// The block command of `intents` when they are one block command for
    /// terminal 3 of tab 7, the pane of these tests.
    fn sent_command(intents: &[TerminalIntent]) -> Option<&BlockCommand> {
        match intents {
            [
                TerminalIntent::RunBlockCommand {
                    tab_id: 7,
                    terminal_id: 3,
                    command,
                },
            ] => Some(command),
            _ => None,
        }
    }

    #[test]
    fn without_a_selected_block_no_block_item_is_offered() {
        let (_, pane) = pane_grid::State::new(());
        let themes = ThemeManager::new();
        let mut ui = simulator(view(PaneContextMenuProps {
            tab_id: 7,
            pane,
            cursor: Point::new(40.0, 40.0),
            grid_size: Size::new(1024.0, 768.0),
            terminal_id: 3,
            block: None,
            theme: ThemeProps::new(themes.current()),
        }));

        assert!(ui.find(i18n::t(Key::CopyContent)).is_err());
        assert!(ui.find(i18n::t(Key::RunAgain)).is_err());
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
            block: None,
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
            block: None,
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

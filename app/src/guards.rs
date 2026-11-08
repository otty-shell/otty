//! What input from the user does while a menu is open or an inline edit runs.
//! These checks only see input: what follows from it, and what arrives on its
//! own, skips them (see `update_internal` in `update.rs`), so it can not get
//! lost.

use crate::router::AppEvent;
use crate::widgets::chrome::{ChromeEvent, ChromeIntent};
use crate::widgets::explorer::{ExplorerEvent, ExplorerIntent};
use crate::widgets::modes::{ModesEvent, ModesIntent};
use crate::widgets::quick_launch::{QuickLaunchEvent, QuickLaunchIntent};
use crate::widgets::sidebar::{SidebarEvent, SidebarIntent};
use crate::widgets::terminal_workspace::{
    TerminalWorkspaceEvent, TerminalWorkspaceIntent,
};

/// Determines how the event loop should treat input from the user when a
/// context menu is open.
#[derive(Debug, Clone, Copy)]
pub(crate) enum MenuGuard {
    /// Let the event pass through to normal dispatch.
    Allow,
    /// Silently drop the event without closing the menu.
    Ignore,
    /// Close all context menus; the event itself does nothing else.
    Dismiss,
    /// Close all context menus, then dispatch the event.
    DismissAndRun,
}

/// Classify input from the user when at least one context menu is open.
pub(crate) fn context_menu_guard(event: &AppEvent) -> MenuGuard {
    use MenuGuard::*;

    match event {
        AppEvent::Sidebar(SidebarEvent::Intent(event)) => {
            use SidebarIntent as E;
            match event {
                E::HeaderMenuDismiss | E::WorkspaceCursorMoved { .. } => Allow,
                E::HeaderMenuOpen(_) => Ignore,
                _ => Dismiss,
            }
        },
        AppEvent::QuickLaunch(QuickLaunchEvent::Intent(event)) => {
            use QuickLaunchIntent as E;
            match event {
                E::ContextMenuDismiss
                | E::ContextMenuAction(_)
                | E::InlineEditChanged(_)
                | E::InlineEditSubmit
                | E::CursorMoved { .. } => Allow,
                _ => Ignore,
            }
        },
        AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(event)) => {
            use TerminalWorkspaceIntent as E;
            match event {
                E::CloseContextMenu { .. }
                | E::CopySelection { .. }
                | E::PasteIntoPrompt { .. }
                | E::CopySelectedBlockContent { .. }
                | E::CopySelectedBlockPrompt { .. }
                | E::CopySelectedBlockCommand { .. }
                | E::SplitPane { .. }
                | E::ClosePane { .. }
                | E::ContextMenuInput { .. }
                | E::Widget(_)
                | E::PaneGridCursorMoved { .. } => Allow,
                E::OpenContextMenu { .. } | E::PaneClicked { .. } => Ignore,
                _ => Dismiss,
            }
        },
        // The first press on a panel button or the palette button only closes
        // the menu; opening settings closes it and goes on.
        AppEvent::Chrome(ChromeEvent::Intent(event)) => {
            use ChromeIntent as E;
            match event {
                E::ToggleLeftPanel
                | E::ToggleFilesPanel
                | E::ToggleFocusMode
                | E::OpenCommandPalette => Dismiss,
                E::OpenSettings => DismissAndRun,
                _ => Allow,
            }
        },
        AppEvent::Explorer(ExplorerEvent::Intent(
            ExplorerIntent::NodeHovered { .. },
        )) => Allow,
        AppEvent::Tabs(_) => DismissAndRun,
        // The All modes menu is one of the menus: its rows, pin toggles and
        // dismiss layer send these. Choosing a mode closes every menu.
        AppEvent::Modes(ModesEvent::Intent(ModesIntent::Select(_))) => {
            DismissAndRun
        },
        AppEvent::Modes(_) => Allow,
        AppEvent::ResizeWindow(_) => Allow,
        // A shortcut, whichever key route it took, closes the menus and
        // runs. Other keys no widget took do nothing under a menu.
        AppEvent::Command(_) => DismissAndRun,
        AppEvent::Keyboard(_) => Ignore,
        _ => Dismiss,
    }
}

/// Return `true` when the event is one of the header menu items (New session
/// or Launches): open a terminal, split the pane, run a launch, create a
/// launch, or create a folder.
///
/// The app closes the menu and runs these instead of asking the context
/// menu guard, which would drop the launch intents.
pub(crate) fn is_header_menu_choice(event: &AppEvent) -> bool {
    use crate::widgets::quick_launch::{QuickLaunchEvent, QuickLaunchIntent};
    use crate::widgets::tabs::{TabsEvent, TabsIntent};
    use crate::widgets::terminal_workspace::{
        TerminalWorkspaceEvent, TerminalWorkspaceIntent,
    };

    matches!(
        event,
        AppEvent::Tabs(TabsEvent::Intent(TabsIntent::OpenTerminalTab))
            | AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
                TerminalWorkspaceIntent::SplitPane { .. }
            ))
            | AppEvent::QuickLaunch(QuickLaunchEvent::Intent(
                QuickLaunchIntent::LaunchNode { .. }
                    | QuickLaunchIntent::HeaderCreateCommand
                    | QuickLaunchIntent::HeaderCreateFolder
            ))
    )
}

/// Return `true` when the event is a press of the Esc key that no widget
/// took. The app closes the All modes menu with it.
pub(crate) fn is_escape_press(event: &AppEvent) -> bool {
    use iced::keyboard::key::Named;
    use iced::keyboard::{Event, Key};

    matches!(
        event,
        AppEvent::Keyboard(Event::KeyPressed {
            key: Key::Named(Named::Escape),
            ..
        })
    )
}

/// Return `true` when an active inline-edit should be cancelled before the
/// input is dispatched: anything but typing into the edit, pointer moves,
/// terminal events and key presses. A shortcut cancels it: it arrives as a
/// command.
pub(crate) fn inline_edit_guard(event: &AppEvent) -> bool {
    match event {
        AppEvent::QuickLaunch(QuickLaunchEvent::Intent(event)) => {
            use QuickLaunchIntent as E;
            !matches!(
                event,
                E::InlineEditChanged(_)
                    | E::InlineEditSubmit
                    | E::CursorMoved { .. }
                    | E::NodeHovered { .. }
            )
        },
        AppEvent::Sidebar(SidebarEvent::Intent(
            SidebarIntent::WorkspaceCursorMoved { .. },
        )) => false,
        AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
            TerminalWorkspaceIntent::Widget(_)
            | TerminalWorkspaceIntent::PaneGridCursorMoved { .. },
        )) => false,
        AppEvent::Explorer(ExplorerEvent::Intent(
            ExplorerIntent::NodeHovered { .. },
        ))
        | AppEvent::Chrome(ChromeEvent::Intent(ChromeIntent::MoveOnTitleBar)) => {
            false
        },
        AppEvent::Keyboard(_) => false,
        _ => true,
    }
}

/// Return `true` when the event carries text a terminal sends to its shell
/// (typed keys, a paste or composed input). The app drops it while the
/// command palette is open, so keys meant for the palette never reach a
/// shell.
pub(crate) fn is_terminal_input(event: &AppEvent) -> bool {
    matches!(
        event,
        AppEvent::TerminalWorkspace(
            crate::widgets::terminal_workspace::TerminalWorkspaceEvent::Intent(
                crate::widgets::terminal_workspace::TerminalWorkspaceIntent::Widget(
                    otty_ui_term::Event::Write { .. }
                )
            )
        )
    )
}

#[cfg(test)]
mod tests {
    use iced::keyboard::key::{Code, Named, NativeCode, Physical};
    use iced::keyboard::{Event, Key, Location, Modifiers};

    use super::{
        MenuGuard, context_menu_guard, inline_edit_guard, is_escape_press,
        is_header_menu_choice, is_terminal_input,
    };
    use crate::commands::AppCommand;
    use crate::domain::mode::Mode;
    use crate::router::AppEvent;
    use crate::shortcuts::AppShortcut;
    use crate::widgets::chrome::{ChromeEvent, ChromeIntent};
    use crate::widgets::containers::actions::RowAction;
    use crate::widgets::containers::inspector::InspectorTab;
    use crate::widgets::containers::{ContainersEvent, ContainersIntent};
    use crate::widgets::explorer::{ExplorerEvent, ExplorerIntent};
    use crate::widgets::modes::{ModesEvent, ModesIntent};
    use crate::widgets::quick_launch::{QuickLaunchEvent, QuickLaunchIntent};
    use crate::widgets::sidebar::types::HeaderMenu;
    use crate::widgets::sidebar::{SidebarEvent, SidebarIntent};
    use crate::widgets::tabs::{TabsEvent, TabsIntent};
    use crate::widgets::terminal_workspace::{
        TerminalWorkspaceEvent, TerminalWorkspaceIntent,
    };

    fn terminal_widget_event(
        event: otty_ui_term::Event<AppShortcut>,
    ) -> AppEvent {
        AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
            TerminalWorkspaceIntent::Widget(event),
        ))
    }

    #[test]
    fn text_a_terminal_writes_to_its_shell_is_terminal_input() {
        let event = terminal_widget_event(otty_ui_term::Event::Write {
            id: 1,
            data: b"ls".to_vec(),
        });

        assert!(is_terminal_input(&event));
    }

    #[test]
    fn other_terminal_and_app_events_are_not_terminal_input() {
        let events = [
            terminal_widget_event(otty_ui_term::Event::Redraw { id: 1 }),
            terminal_widget_event(
                otty_ui_term::Event::BindingActionDispatched {
                    id: 1,
                    action: AppShortcut::CommandPalette,
                },
            ),
            AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
                TerminalWorkspaceIntent::FocusActive,
            )),
            AppEvent::SyncTerminalGridSizes,
        ];

        for event in events {
            assert!(!is_terminal_input(&event));
        }
    }

    fn key_press(key: Key, physical_key: Physical) -> AppEvent {
        AppEvent::Keyboard(Event::KeyPressed {
            key: key.clone(),
            modified_key: key,
            physical_key,
            location: Location::Standard,
            modifiers: Modifiers::empty(),
            text: None,
            repeat: false,
        })
    }

    #[test]
    fn a_pressed_escape_key_is_an_escape_press() {
        let event =
            key_press(Key::Named(Named::Escape), Physical::Code(Code::Escape));

        assert!(is_escape_press(&event));
    }

    #[test]
    fn other_keys_and_a_released_escape_are_not_an_escape_press() {
        let letter = key_press(
            Key::Character("a".into()),
            Physical::Unidentified(NativeCode::Unidentified),
        );
        let released = AppEvent::Keyboard(Event::KeyReleased {
            key: Key::Named(Named::Escape),
            modified_key: Key::Named(Named::Escape),
            physical_key: Physical::Code(Code::Escape),
            location: Location::Standard,
            modifiers: Modifiers::empty(),
        });

        assert!(!is_escape_press(&letter));
        assert!(!is_escape_press(&released));
        assert!(!is_escape_press(&AppEvent::SyncTerminalGridSizes));
    }

    #[test]
    fn given_menu_open_when_modes_events_arrive_then_they_are_allowed() {
        let events = [
            AppEvent::Modes(ModesEvent::Intent(ModesIntent::CloseMenu)),
            AppEvent::Modes(ModesEvent::Intent(ModesIntent::Pin(
                Mode::Containers,
            ))),
        ];

        for event in events {
            assert!(matches!(context_menu_guard(&event), MenuGuard::Allow));
        }
    }

    #[test]
    fn choosing_a_mode_or_pressing_a_tab_closes_the_menus_and_goes_on() {
        let events = [
            AppEvent::Modes(ModesEvent::Intent(ModesIntent::Select(
                Mode::Containers,
            ))),
            AppEvent::Tabs(TabsEvent::Intent(TabsIntent::ActivateTab {
                tab_id: 1,
            })),
            AppEvent::Chrome(ChromeEvent::Intent(ChromeIntent::OpenSettings)),
        ];

        for event in events {
            assert!(matches!(
                context_menu_guard(&event),
                MenuGuard::DismissAndRun
            ));
        }
    }

    #[test]
    fn a_panel_or_palette_button_only_closes_the_menus() {
        let presses = [
            ChromeIntent::ToggleLeftPanel,
            ChromeIntent::ToggleFilesPanel,
            ChromeIntent::ToggleFocusMode,
            ChromeIntent::OpenCommandPalette,
        ];

        for press in presses {
            let event = AppEvent::Chrome(ChromeEvent::Intent(press));

            assert!(matches!(context_menu_guard(&event), MenuGuard::Dismiss));
        }
    }

    #[test]
    fn a_window_button_leaves_the_menus_open() {
        let event =
            AppEvent::Chrome(ChromeEvent::Intent(ChromeIntent::MinimizeWindow));

        assert!(matches!(context_menu_guard(&event), MenuGuard::Allow));
    }

    #[test]
    fn a_key_press_that_is_not_a_shortcut_keeps_an_inline_edit() {
        // A shortcut key reaches the checks as its command instead.
        let plain =
            key_press(Key::Character("a".into()), Physical::Code(Code::KeyA));

        assert!(!inline_edit_guard(&plain));
    }

    #[test]
    fn a_click_in_the_containers_views_closes_menus_and_inline_edits() {
        let containers =
            |intent| AppEvent::Containers(ContainersEvent::Intent(intent));
        let clicks = [
            containers(ContainersIntent::ToggleEngine(
                otty_containers::EngineAddress::Tcp(String::from("h:1")),
            )),
            containers(ContainersIntent::Refresh),
            containers(ContainersIntent::SelectRow(0)),
            containers(ContainersIntent::SelectInspectorTab(
                InspectorTab::Logs,
            )),
            containers(ContainersIntent::Act(RowAction::Stop)),
            containers(ContainersIntent::Prune),
        ];

        for event in clicks {
            assert!(matches!(context_menu_guard(&event), MenuGuard::Dismiss));
            assert!(inline_edit_guard(&event));
        }
    }

    #[test]
    fn given_menu_open_when_menu_open_intent_arrives_then_it_is_ignored() {
        let guard =
            context_menu_guard(&AppEvent::Sidebar(SidebarEvent::Intent(
                SidebarIntent::HeaderMenuOpen(HeaderMenu::Sessions),
            )));
        assert!(matches!(guard, MenuGuard::Ignore));
    }

    #[test]
    fn given_menu_open_when_menu_dismiss_intent_arrives_then_it_is_allowed() {
        let guard = context_menu_guard(&AppEvent::Sidebar(
            SidebarEvent::Intent(SidebarIntent::HeaderMenuDismiss),
        ));
        assert!(matches!(guard, MenuGuard::Allow));
    }

    #[test]
    fn given_menu_choices_when_checked_then_each_one_is_a_header_menu_choice() {
        let open_terminal =
            AppEvent::Tabs(TabsEvent::Intent(TabsIntent::OpenTerminalTab));
        let launch = AppEvent::QuickLaunch(QuickLaunchEvent::Intent(
            QuickLaunchIntent::LaunchNode {
                path: vec![String::from("Demo")],
            },
        ));
        let new_launch = AppEvent::QuickLaunch(QuickLaunchEvent::Intent(
            QuickLaunchIntent::HeaderCreateCommand,
        ));
        let new_folder = AppEvent::QuickLaunch(QuickLaunchEvent::Intent(
            QuickLaunchIntent::HeaderCreateFolder,
        ));

        let (_panes, pane) = iced::widget::pane_grid::State::new(());
        let split =
            AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
                TerminalWorkspaceIntent::SplitPane {
                    tab_id: 0,
                    pane,
                    axis: iced::widget::pane_grid::Axis::Vertical,
                },
            ));

        assert!(is_header_menu_choice(&open_terminal));
        assert!(is_header_menu_choice(&split));
        assert!(is_header_menu_choice(&launch));
        assert!(is_header_menu_choice(&new_launch));
        assert!(is_header_menu_choice(&new_folder));
    }

    #[test]
    fn given_other_events_when_checked_then_they_are_not_menu_choices() {
        let close_tab =
            AppEvent::Tabs(TabsEvent::Intent(TabsIntent::CloseTab {
                tab_id: 1,
            }));
        let pressed = AppEvent::QuickLaunch(QuickLaunchEvent::Intent(
            QuickLaunchIntent::NodePressed {
                path: vec![String::from("Demo")],
            },
        ));

        assert!(!is_header_menu_choice(&close_tab));
        assert!(!is_header_menu_choice(&pressed));
        assert!(!is_header_menu_choice(&AppEvent::IcedReady));
    }

    #[test]
    fn the_right_panel_button_still_closes_menus_and_inline_edits() {
        let event =
            AppEvent::Sidebar(SidebarEvent::Intent(SidebarIntent::ToggleRight));

        assert!(matches!(context_menu_guard(&event), MenuGuard::Dismiss));
        assert!(inline_edit_guard(&event));
    }

    #[test]
    fn hovering_a_files_row_keeps_menus_and_inline_edits() {
        // A row sends the path when the pointer enters it, and no path when
        // the pointer leaves it or the whole tree.
        let paths = [Some(vec![String::from("src")]), None];

        for path in paths {
            let event = AppEvent::Explorer(ExplorerEvent::Intent(
                ExplorerIntent::NodeHovered { path },
            ));

            assert!(matches!(context_menu_guard(&event), MenuGuard::Allow));
            assert!(!inline_edit_guard(&event));
        }
    }

    #[test]
    fn moving_over_the_title_bar_keeps_menus_and_inline_edits() {
        let event =
            AppEvent::Chrome(ChromeEvent::Intent(ChromeIntent::MoveOnTitleBar));

        assert!(matches!(context_menu_guard(&event), MenuGuard::Allow));
        assert!(!inline_edit_guard(&event));
    }

    #[test]
    fn pressing_a_files_row_still_closes_menus_and_inline_edits() {
        let event = AppEvent::Explorer(ExplorerEvent::Intent(
            ExplorerIntent::NodePressed {
                path: vec![String::from("src")],
            },
        ));

        assert!(matches!(context_menu_guard(&event), MenuGuard::Dismiss));
        assert!(inline_edit_guard(&event));
    }

    #[test]
    fn pressing_the_title_bar_still_cancels_an_inline_edit() {
        let event =
            AppEvent::Chrome(ChromeEvent::Intent(ChromeIntent::PressTitleBar));

        assert!(inline_edit_guard(&event));
    }

    #[test]
    fn a_command_under_an_open_menu_closes_the_menus_and_goes_on() {
        let event = AppEvent::Command(AppCommand::ToggleFocusMode);

        assert!(matches!(
            context_menu_guard(&event),
            MenuGuard::DismissAndRun
        ));
    }

    #[test]
    fn a_command_cancels_an_inline_edit() {
        let event = AppEvent::Command(AppCommand::ToggleFilesPanel);

        assert!(inline_edit_guard(&event));
    }
}

use crate::events::AppEvent;

/// Determines how the event loop should treat an incoming event when a
/// context menu is open.
#[derive(Debug, Clone, Copy)]
pub(crate) enum MenuGuard {
    /// Let the event pass through to normal dispatch.
    Allow,
    /// Silently drop the event without closing the menu.
    Ignore,
    /// Close all context menus before dispatching.
    Dismiss,
}

/// Classify an incoming event when at least one context menu is open.
pub(crate) fn context_menu_guard(event: &AppEvent) -> MenuGuard {
    use MenuGuard::*;

    match event {
        AppEvent::Sidebar(crate::widgets::sidebar::SidebarEvent::Intent(
            event,
        )) => {
            use crate::widgets::sidebar::SidebarIntent as E;
            match event {
                E::HeaderMenuDismiss => Allow,
                E::HeaderMenuOpen(_) => Ignore,
                E::WorkspaceCursorMoved { .. }
                | E::PaneGridCursorMoved { .. } => Allow,
                _ => Dismiss,
            }
        },
        AppEvent::QuickLaunch(
            crate::widgets::quick_launch::QuickLaunchEvent::Intent(event),
        ) => {
            use crate::widgets::quick_launch::QuickLaunchIntent as E;
            match event {
                E::ContextMenuDismiss
                | E::ContextMenuAction(_)
                | E::InlineEditChanged(_)
                | E::InlineEditSubmit
                | E::CancelInlineEdit => Allow,
                E::CursorMoved { .. } => Allow,
                E::Tick
                | E::PersistCompleted
                | E::PersistFailed(_)
                | E::SetupCompleted(_)
                | E::WizardSaveRequested(_) => Allow,
                _ => Ignore,
            }
        },
        AppEvent::TerminalWorkspace(
            crate::widgets::terminal_workspace::TerminalWorkspaceEvent::Intent(
                event,
            ),
        ) => {
            use crate::widgets::terminal_workspace::TerminalWorkspaceIntent as E;
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
                | E::PaneGridCursorMoved { .. }
                | E::SyncPaneGridSize => Allow,
                E::OpenContextMenu { .. } | E::PaneClicked { .. } => Ignore,
                _ => Dismiss,
            }
        },
        AppEvent::Sidebar(crate::widgets::sidebar::SidebarEvent::Effect(_))
        | AppEvent::Chrome(crate::widgets::chrome::ChromeEvent::Effect(_))
        | AppEvent::Tabs(crate::widgets::tabs::TabsEvent::Effect(_))
        | AppEvent::QuickLaunch(
            crate::widgets::quick_launch::QuickLaunchEvent::Effect(_),
        )
        | AppEvent::TerminalWorkspace(
            crate::widgets::terminal_workspace::TerminalWorkspaceEvent::Effect(
                _,
            ),
        ) => Allow,
        AppEvent::Settings(
            crate::widgets::settings::SettingsEvent::Effect(_),
        )
        | AppEvent::Settings(
            crate::widgets::settings::SettingsEvent::Intent(
                crate::widgets::settings::SettingsIntent::ReloadLoaded(_)
                | crate::widgets::settings::SettingsIntent::ReloadFailed(_)
                | crate::widgets::settings::SettingsIntent::SaveCompleted(_)
                | crate::widgets::settings::SettingsIntent::SaveFailed(_),
            ),
        ) => Allow,
        AppEvent::Explorer(
            crate::widgets::explorer::ExplorerEvent::Effect(_),
        )
        | AppEvent::Explorer(
            crate::widgets::explorer::ExplorerEvent::Intent(
                crate::widgets::explorer::ExplorerIntent::RootLoaded { .. }
                | crate::widgets::explorer::ExplorerIntent::FolderLoaded {
                    ..
                }
                | crate::widgets::explorer::ExplorerIntent::DirectoryChanged {
                    ..
                }
                | crate::widgets::explorer::ExplorerIntent::DirectoryReloaded {
                    ..
                }
                | crate::widgets::explorer::ExplorerIntent::LoadFailed { .. },
            ),
        ) => Allow,
        AppEvent::Explorer(
            crate::widgets::explorer::ExplorerEvent::Intent(
                crate::widgets::explorer::ExplorerIntent::SyncRoot { .. },
            ),
        ) => Allow,
        AppEvent::Window(_) | AppEvent::ResizeWindow(_) => Allow,
        AppEvent::SyncTerminalGridSizes => Allow,
        AppEvent::Keyboard(_) => Ignore,
        AppEvent::Chrome(crate::widgets::chrome::ChromeEvent::Intent(_))
        | AppEvent::Tabs(crate::widgets::tabs::TabsEvent::Intent(_)) => Allow,
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
        AppEvent::Tabs(TabsEvent::Intent(TabsIntent::OpenTerminalTab { .. }))
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

/// Return `true` when an active inline-edit should be cancelled before the
/// event is dispatched.
pub(crate) fn inline_edit_guard(event: &AppEvent) -> bool {
    match event {
        AppEvent::QuickLaunch(
            crate::widgets::quick_launch::QuickLaunchEvent::Intent(event),
        ) => {
            use crate::widgets::quick_launch::QuickLaunchIntent as E;
            !matches!(
                event,
                E::InlineEditChanged(_)
                    | E::InlineEditSubmit
                    | E::CursorMoved { .. }
                    | E::NodeHovered { .. }
                    | E::SetupCompleted(_)
                    | E::PersistCompleted
                    | E::PersistFailed(_)
                    | E::Tick
                    | E::WizardSaveRequested(_)
            )
        },
        AppEvent::QuickLaunch(
            crate::widgets::quick_launch::QuickLaunchEvent::Effect(_),
        ) => false,
        AppEvent::Sidebar(crate::widgets::sidebar::SidebarEvent::Intent(
            event,
        )) => {
            use crate::widgets::sidebar::SidebarIntent as E;
            !matches!(
                event,
                E::WorkspaceCursorMoved { .. } | E::PaneGridCursorMoved { .. }
            )
        },
        AppEvent::TerminalWorkspace(
            crate::widgets::terminal_workspace::TerminalWorkspaceEvent::Intent(
                event,
            ),
        ) => {
            use crate::widgets::terminal_workspace::TerminalWorkspaceIntent as E;
            !matches!(
                event,
                E::Widget(_)
                    | E::PaneGridCursorMoved { .. }
                    | E::SyncPaneGridSize
            )
        },
        AppEvent::TerminalWorkspace(
            crate::widgets::terminal_workspace::TerminalWorkspaceEvent::Effect(
                _,
            ),
        ) => false,
        AppEvent::Tabs(crate::widgets::tabs::TabsEvent::Intent(event)) => {
            use crate::widgets::tabs::TabsIntent as E;
            matches!(event, E::ActivateTab { .. } | E::CloseTab { .. })
        },
        AppEvent::Settings(
            crate::widgets::settings::SettingsEvent::Effect(_),
        )
        | AppEvent::Settings(
            crate::widgets::settings::SettingsEvent::Intent(
                crate::widgets::settings::SettingsIntent::Reload
                | crate::widgets::settings::SettingsIntent::ReloadLoaded(_)
                | crate::widgets::settings::SettingsIntent::ReloadFailed(_)
                | crate::widgets::settings::SettingsIntent::SaveCompleted(_)
                | crate::widgets::settings::SettingsIntent::SaveFailed(_),
            ),
        ) => false,
        AppEvent::Explorer(
            crate::widgets::explorer::ExplorerEvent::Effect(_),
        )
        | AppEvent::Explorer(
            crate::widgets::explorer::ExplorerEvent::Intent(
                crate::widgets::explorer::ExplorerIntent::SyncRoot { .. }
                | crate::widgets::explorer::ExplorerIntent::RootLoaded { .. }
                | crate::widgets::explorer::ExplorerIntent::FolderLoaded {
                    ..
                }
                | crate::widgets::explorer::ExplorerIntent::DirectoryChanged {
                    ..
                }
                | crate::widgets::explorer::ExplorerIntent::DirectoryReloaded {
                    ..
                }
                | crate::widgets::explorer::ExplorerIntent::LoadFailed { .. },
            ),
        ) => false,
        AppEvent::SyncTerminalGridSizes => false,
        AppEvent::Keyboard(_) | AppEvent::Window(_) => false,
        AppEvent::Sidebar(crate::widgets::sidebar::SidebarEvent::Effect(_))
        | AppEvent::Chrome(crate::widgets::chrome::ChromeEvent::Effect(_))
        | AppEvent::Tabs(crate::widgets::tabs::TabsEvent::Effect(_)) => false,
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
    use std::path::PathBuf;

    use super::{
        MenuGuard, context_menu_guard, inline_edit_guard,
        is_header_menu_choice, is_terminal_input,
    };
    use crate::events::AppEvent;
    use crate::shortcuts::AppShortcut;
    use crate::widgets::explorer::{ExplorerEvent, ExplorerIntent};
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
            AppEvent::Tabs(TabsEvent::Intent(TabsIntent::OpenTerminalTab {
                title: String::from("zsh"),
            }));
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
    fn given_sync_terminal_grid_sizes_when_inline_edit_guard_runs_then_edit_is_not_cancelled()
     {
        assert!(!inline_edit_guard(&AppEvent::SyncTerminalGridSizes));
    }

    #[test]
    fn given_explorer_directory_change_when_context_menu_guard_runs_then_event_is_allowed()
     {
        let guard = context_menu_guard(&AppEvent::Explorer(
            ExplorerEvent::Intent(ExplorerIntent::DirectoryChanged {
                directory: PathBuf::from("/tmp/project"),
            }),
        ));

        assert!(matches!(guard, MenuGuard::Allow));
    }

    #[test]
    fn given_explorer_directory_change_when_inline_edit_guard_runs_then_edit_is_not_cancelled()
     {
        let event = AppEvent::Explorer(ExplorerEvent::Intent(
            ExplorerIntent::DirectoryChanged {
                directory: PathBuf::from("/tmp/project"),
            },
        ));

        assert!(!inline_edit_guard(&event));
    }
}

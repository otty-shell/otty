use super::features::quick_launch;
use super::features::terminal::TerminalEvent;
use super::ui::widgets::sidebar_workspace;
use crate::app::Event;

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
pub(crate) fn context_menu_guard(event: &Event) -> MenuGuard {
    use MenuGuard::*;

    match event {
        Event::SidebarWorkspace(
            sidebar_workspace::SidebarWorkspaceEvent::TerminalAddMenuAction(_)
            | sidebar_workspace::SidebarWorkspaceEvent::TerminalAddMenuDismiss,
        ) => Allow,
        Event::SidebarWorkspace(
            sidebar_workspace::SidebarWorkspaceEvent::TerminalAddMenuOpen,
        ) => Ignore,
        Event::SidebarWorkspace(
            sidebar_workspace::SidebarWorkspaceEvent::QuickLaunch(
                quick_launch::QuickLaunchEvent::ContextMenuAction(_)
                | quick_launch::QuickLaunchEvent::ContextMenuDismiss
                | quick_launch::QuickLaunchEvent::CursorMoved { .. },
            ),
        )
        | Event::QuickLaunch(
            quick_launch::QuickLaunchEvent::ContextMenuAction(_)
            | quick_launch::QuickLaunchEvent::ContextMenuDismiss
            | quick_launch::QuickLaunchEvent::SetupCompleted(_)
            | quick_launch::QuickLaunchEvent::Tick
            | quick_launch::QuickLaunchEvent::CursorMoved { .. },
        ) => Allow,
        Event::SidebarWorkspace(
            sidebar_workspace::SidebarWorkspaceEvent::QuickLaunch(_),
        )
        | Event::QuickLaunch(_) => Ignore,
        Event::Terminal(event) => match event {
            TerminalEvent::CloseContextMenu { .. }
            | TerminalEvent::CopySelection { .. }
            | TerminalEvent::PasteIntoPrompt { .. }
            | TerminalEvent::CopySelectedBlockContent { .. }
            | TerminalEvent::CopySelectedBlockPrompt { .. }
            | TerminalEvent::CopySelectedBlockCommand { .. }
            | TerminalEvent::SplitPane { .. }
            | TerminalEvent::ClosePane { .. } => Allow,
            TerminalEvent::Widget(_) => Allow,
            TerminalEvent::PaneGridCursorMoved { .. } => Allow,
            TerminalEvent::OpenContextMenu { .. } => Ignore,
            TerminalEvent::PaneClicked { .. } => Ignore,
            _ => Dismiss,
        },
        Event::SidebarWorkspace(
            sidebar_workspace::SidebarWorkspaceEvent::WorkspaceCursorMoved {
                ..
            },
        )
        | Event::Explorer(_) => Allow,
        Event::ActionBar(_) => Allow,
        Event::Window(_) | Event::ResizeWindow(_) => Allow,
        Event::OpenTerminalTab
        | Event::OpenSettingsTab
        | Event::SyncTerminalGridSizes => Allow,
        Event::SetTabTitle { .. } => Allow,
        Event::CloseTabRequested { .. } => Allow,
        Event::Keyboard(_) => Ignore,
        _ => Dismiss,
    }
}

/// Return `true` when an active inline-edit should be cancelled before the
/// event is dispatched.
pub(crate) fn inline_edit_guard(event: &Event) -> bool {
    use quick_launch::QuickLaunchEvent;

    match event {
        Event::SidebarWorkspace(
            sidebar_workspace::SidebarWorkspaceEvent::QuickLaunch(
                quick_launches_event,
            ),
        ) => !matches!(
            quick_launches_event,
            QuickLaunchEvent::InlineEditChanged(_)
                | QuickLaunchEvent::InlineEditSubmit
                | QuickLaunchEvent::CursorMoved { .. }
                | QuickLaunchEvent::NodeHovered { .. }
        ),
        Event::QuickLaunch(quick_launches_event) => !matches!(
            quick_launches_event,
            QuickLaunchEvent::InlineEditChanged(_)
                | QuickLaunchEvent::InlineEditSubmit
                | QuickLaunchEvent::CursorMoved { .. }
                | QuickLaunchEvent::NodeHovered { .. }
                | QuickLaunchEvent::SetupCompleted(_)
                | QuickLaunchEvent::Tick
        ),
        Event::CloseTabRequested { .. } => false,
        Event::OpenTerminalTab
        | Event::OpenSettingsTab
        | Event::SyncTerminalGridSizes => false,
        Event::SidebarWorkspace(
            sidebar_workspace::SidebarWorkspaceEvent::WorkspaceCursorMoved {
                ..
            },
        ) => false,
        Event::Terminal(event) => !matches!(
            event,
            TerminalEvent::Widget(_)
                | TerminalEvent::PaneGridCursorMoved { .. }
        ),
        Event::Keyboard(_) | Event::Window(_) => false,
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use iced::Point;
    use iced::widget::pane_grid;

    use super::{MenuGuard, context_menu_guard, inline_edit_guard};
    use crate::app::Event;
    use crate::features::quick_launch::QuickLaunchEvent;
    use crate::features::terminal::TerminalEvent;
    use crate::ui::widgets::sidebar_workspace::SidebarWorkspaceEvent;

    #[test]
    fn given_terminal_pane_click_when_context_menu_guard_runs_then_event_is_ignored()
     {
        let (_grid, pane) = pane_grid::State::new(0_u64);
        let guard =
            context_menu_guard(&Event::Terminal(TerminalEvent::PaneClicked {
                tab_id: 1,
                pane,
            }));
        assert!(matches!(guard, MenuGuard::Ignore));
    }

    #[test]
    fn given_terminal_open_context_menu_when_context_menu_guard_runs_then_event_is_ignored()
     {
        let (_grid, pane) = pane_grid::State::new(0_u64);
        let guard = context_menu_guard(&Event::Terminal(
            TerminalEvent::OpenContextMenu {
                tab_id: 1,
                pane,
                terminal_id: 10,
            },
        ));
        assert!(matches!(guard, MenuGuard::Ignore));
    }

    #[test]
    fn given_sidebar_quick_launch_node_release_when_context_menu_guard_runs_then_event_is_ignored()
     {
        let guard = context_menu_guard(&Event::SidebarWorkspace(
            SidebarWorkspaceEvent::QuickLaunch(
                QuickLaunchEvent::NodeReleased {
                    path: vec![String::from("node")],
                },
            ),
        ));
        assert!(matches!(guard, MenuGuard::Ignore));
    }

    #[test]
    fn given_sidebar_add_menu_open_when_context_menu_guard_runs_then_event_is_ignored()
     {
        let guard = context_menu_guard(&Event::SidebarWorkspace(
            SidebarWorkspaceEvent::TerminalAddMenuOpen,
        ));
        assert!(matches!(guard, MenuGuard::Ignore));
    }

    #[test]
    fn given_quick_launch_cursor_move_when_context_menu_guard_runs_then_event_is_allowed()
     {
        let guard = context_menu_guard(&Event::SidebarWorkspace(
            SidebarWorkspaceEvent::QuickLaunch(QuickLaunchEvent::CursorMoved {
                position: Point::new(10.0, 20.0),
            }),
        ));
        assert!(matches!(guard, MenuGuard::Allow));
    }

    #[test]
    fn given_set_tab_title_when_context_menu_guard_runs_then_event_is_allowed()
    {
        let guard = context_menu_guard(&Event::SetTabTitle {
            tab_id: 1,
            title: String::from("title"),
        });
        assert!(matches!(guard, MenuGuard::Allow));
    }

    #[test]
    fn given_open_terminal_tab_when_context_menu_guard_runs_then_event_is_allowed()
     {
        let guard = context_menu_guard(&Event::OpenTerminalTab);
        assert!(matches!(guard, MenuGuard::Allow));
    }

    #[test]
    fn given_sync_terminal_grid_sizes_when_inline_edit_guard_runs_then_edit_is_not_cancelled()
     {
        assert!(!inline_edit_guard(&Event::SyncTerminalGridSizes));
    }
}

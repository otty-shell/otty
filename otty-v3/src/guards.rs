use crate::app::AppEvent;

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
        AppEvent::SidebarUi(event) => {
            use crate::widgets::sidebar::SidebarEvent as E;
            match event {
                E::AddMenuDismiss
                | E::AddMenuCreateTab
                | E::AddMenuCreateCommand
                | E::AddMenuCreateFolder => Allow,
                E::AddMenuOpen => Ignore,
                E::WorkspaceCursorMoved { .. }
                | E::PaneGridCursorMoved { .. } => Allow,
                _ => Dismiss,
            }
        },
        AppEvent::QuickLaunchUi(event) => {
            use crate::widgets::quick_launch::QuickLaunchEvent as E;
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
        AppEvent::TerminalWorkspaceUi(event) => {
            use crate::widgets::terminal_workspace::TerminalWorkspaceEvent as E;
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
        AppEvent::SidebarEffect(_)
        | AppEvent::ChromeEffect(_)
        | AppEvent::TabsEffect(_)
        | AppEvent::QuickLaunchEffect(_)
        | AppEvent::TerminalWorkspaceEffect(_) => Allow,
        AppEvent::SidebarCommand(_)
        | AppEvent::ChromeCommand(_)
        | AppEvent::TabsCommand(_)
        | AppEvent::QuickLaunchCommand(_)
        | AppEvent::TerminalWorkspaceCommand(_)
        | AppEvent::ExplorerCommand(_)
        | AppEvent::SettingsCommand(_) => Allow,
        AppEvent::Window(_) | AppEvent::ResizeWindow(_) => Allow,
        AppEvent::OpenTerminalTab
        | AppEvent::OpenSettingsTab
        | AppEvent::OpenQuickLaunchWizardCreateTab { .. }
        | AppEvent::OpenQuickLaunchWizardEditTab { .. }
        | AppEvent::OpenQuickLaunchCommandTerminalTab { .. }
        | AppEvent::OpenQuickLaunchErrorTab { .. }
        | AppEvent::OpenFileTerminalTab { .. }
        | AppEvent::CloseTab { .. }
        | AppEvent::SyncTerminalGridSizes => Allow,
        AppEvent::Keyboard(_) => Ignore,
        AppEvent::ChromeUi(_) | AppEvent::TabsUi(_) => Allow,
        _ => Dismiss,
    }
}

/// Return `true` when an active inline-edit should be cancelled before the
/// event is dispatched.
pub(crate) fn inline_edit_guard(event: &AppEvent) -> bool {
    match event {
        AppEvent::QuickLaunchUi(event) => {
            use crate::widgets::quick_launch::QuickLaunchEvent as E;
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
        AppEvent::QuickLaunchEffect(_) => false,
        AppEvent::SidebarUi(event) => {
            use crate::widgets::sidebar::SidebarEvent as E;
            !matches!(
                event,
                E::WorkspaceCursorMoved { .. } | E::PaneGridCursorMoved { .. }
            )
        },
        AppEvent::TerminalWorkspaceUi(event) => {
            use crate::widgets::terminal_workspace::TerminalWorkspaceEvent as E;
            !matches!(event, E::Widget(_) | E::PaneGridCursorMoved { .. })
        },
        AppEvent::TerminalWorkspaceEffect(_) => false,
        AppEvent::SidebarCommand(_)
        | AppEvent::ChromeCommand(_)
        | AppEvent::TabsCommand(_)
        | AppEvent::QuickLaunchCommand(_)
        | AppEvent::TerminalWorkspaceCommand(_)
        | AppEvent::ExplorerCommand(_)
        | AppEvent::SettingsCommand(_) => false,
        AppEvent::OpenTerminalTab
        | AppEvent::OpenSettingsTab
        | AppEvent::OpenQuickLaunchWizardCreateTab { .. }
        | AppEvent::OpenQuickLaunchWizardEditTab { .. }
        | AppEvent::OpenQuickLaunchCommandTerminalTab { .. }
        | AppEvent::OpenQuickLaunchErrorTab { .. }
        | AppEvent::OpenFileTerminalTab { .. }
        | AppEvent::CloseTab { .. }
        | AppEvent::SyncTerminalGridSizes => false,
        AppEvent::Keyboard(_) | AppEvent::Window(_) => false,
        AppEvent::SidebarEffect(_)
        | AppEvent::ChromeEffect(_)
        | AppEvent::TabsEffect(_) => false,
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::{MenuGuard, context_menu_guard, inline_edit_guard};
    use crate::app::AppEvent;
    use crate::widgets::sidebar::SidebarEvent;

    #[test]
    fn given_add_menu_open_when_context_menu_guard_runs_then_event_is_ignored()
    {
        let guard =
            context_menu_guard(&AppEvent::SidebarUi(SidebarEvent::AddMenuOpen));
        assert!(matches!(guard, MenuGuard::Ignore));
    }

    #[test]
    fn given_open_terminal_tab_when_context_menu_guard_runs_then_event_is_allowed()
     {
        let guard = context_menu_guard(&AppEvent::OpenTerminalTab);
        assert!(matches!(guard, MenuGuard::Allow));
    }

    #[test]
    fn given_sync_terminal_grid_sizes_when_inline_edit_guard_runs_then_edit_is_not_cancelled()
     {
        assert!(!inline_edit_guard(&AppEvent::SyncTerminalGridSizes));
    }
}

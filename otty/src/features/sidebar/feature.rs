use iced::Task;

use super::event::SidebarEvent;
use super::model::{SidebarItem, SidebarPane};
use super::state::SidebarState;
use crate::app::Event as AppEvent;
use crate::features::quick_launch::QuickLaunchEvent;
use crate::ui::widgets::{sidebar_menu, sidebar_workspace};

const SIDEBAR_MIN_TAB_CONTENT_RATIO: f32 = 0.2;

/// Sidebar feature root that owns sidebar state and reduction logic.
pub(crate) struct SidebarFeature {
    state: SidebarState,
}

impl SidebarFeature {
    /// Construct sidebar feature with default layout state.
    pub(crate) fn new() -> Self {
        Self {
            state: SidebarState::new(),
        }
    }

    /// Return the currently selected sidebar menu item.
    pub(crate) fn active_item(&self) -> SidebarItem {
        self.state.active_item()
    }

    /// Return whether the sidebar rail and workspace are hidden.
    pub(crate) fn is_hidden(&self) -> bool {
        self.state.is_hidden()
    }

    /// Return whether the workspace pane is currently open.
    pub(crate) fn is_workspace_open(&self) -> bool {
        self.state.is_workspace_open()
    }

    /// Return read-only access to pane-grid slots for rendering.
    pub(crate) fn panes(&self) -> &iced::widget::pane_grid::State<SidebarPane> {
        self.state.panes()
    }

    /// Return the current cursor position snapshot.
    pub(crate) fn cursor(&self) -> iced::Point {
        self.state.cursor()
    }

    /// Return add-menu cursor anchor when the menu is open.
    pub(crate) fn add_menu_cursor(&self) -> Option<iced::Point> {
        self.state.add_menu_cursor()
    }

    /// Return whether the add-menu overlay is currently open.
    pub(crate) fn has_add_menu_open(&self) -> bool {
        self.state.has_add_menu_open()
    }

    /// Return whether the user is actively resizing the sidebar split.
    pub(crate) fn is_resizing(&self) -> bool {
        self.state.is_resizing()
    }

    /// Return effective workspace ratio used for content sizing.
    pub(crate) fn effective_workspace_ratio(&self) -> f32 {
        self.state.effective_workspace_ratio()
    }
}

impl SidebarFeature {
    /// Reduce sidebar events into state updates and routed app tasks.
    pub(crate) fn reduce(
        &mut self,
        event: SidebarEvent,
        _ctx: &(),
    ) -> Task<AppEvent> {
        match event {
            SidebarEvent::Menu(event) => self.reduce_menu_event(event),
            SidebarEvent::Workspace(event) => {
                self.reduce_workspace_event(event)
            },
            SidebarEvent::ToggleVisibility => {
                self.state.toggle_visibility();
                Task::done(AppEvent::SyncTerminalGridSizes)
            },
            SidebarEvent::PaneGridCursorMoved { position } => {
                self.state.update_cursor(position);
                Task::none()
            },
            SidebarEvent::DismissAddMenu => {
                self.state.dismiss_add_menu();
                Task::none()
            },
        }
    }

    fn reduce_menu_event(
        &mut self,
        event: sidebar_menu::SidebarMenuEvent,
    ) -> Task<AppEvent> {
        match event {
            sidebar_menu::SidebarMenuEvent::SelectItem(item) => {
                let canonical = match item {
                    sidebar_menu::SidebarMenuItem::Terminal => {
                        SidebarItem::Terminal
                    },
                    sidebar_menu::SidebarMenuItem::Explorer => {
                        SidebarItem::Explorer
                    },
                };
                self.state.set_active_item(canonical);

                if self
                    .state
                    .ensure_workspace_open(max_sidebar_workspace_ratio())
                {
                    Task::done(AppEvent::SyncTerminalGridSizes)
                } else {
                    Task::none()
                }
            },
            sidebar_menu::SidebarMenuEvent::OpenSettings => {
                Task::done(AppEvent::OpenSettingsTab)
            },
            sidebar_menu::SidebarMenuEvent::ToggleWorkspace => {
                let _ =
                    self.state.toggle_workspace(max_sidebar_workspace_ratio());
                Task::done(AppEvent::SyncTerminalGridSizes)
            },
            sidebar_menu::SidebarMenuEvent::Resized(event) => {
                self.state.mark_resizing();
                let mut tasks = vec![Task::done(AppEvent::QuickLaunch(
                    QuickLaunchEvent::ResetInteractionState,
                ))];

                if self
                    .state
                    .apply_resize(event, max_sidebar_workspace_ratio())
                {
                    tasks.push(
                        Task::done(AppEvent::SyncTerminalGridSizes)
                    );
                }

                Task::batch(tasks)
            },
        }
    }

    fn reduce_workspace_event(
        &mut self,
        event: sidebar_workspace::SidebarWorkspaceEvent,
    ) -> Task<AppEvent> {
        match event {
            sidebar_workspace::SidebarWorkspaceEvent::TerminalAddMenuOpen => {
                self.state.open_add_menu();
                Task::none()
            },
            sidebar_workspace::SidebarWorkspaceEvent::TerminalAddMenuDismiss => {
                self.state.dismiss_add_menu();
                Task::none()
            },
            sidebar_workspace::SidebarWorkspaceEvent::TerminalAddMenuAction(
                action,
            ) => {
                self.state.dismiss_add_menu();
                match action {
                    sidebar_workspace::SidebarWorkspaceAddMenuAction::CreateTab => {
                        Task::done(AppEvent::OpenTerminalTab)
                    },
                    sidebar_workspace::SidebarWorkspaceAddMenuAction::CreateCommand => {
                        Task::done(AppEvent::QuickLaunch(
                            QuickLaunchEvent::HeaderCreateCommand,
                        ))
                    },
                    sidebar_workspace::SidebarWorkspaceAddMenuAction::CreateFolder => {
                        Task::done(AppEvent::QuickLaunch(
                            QuickLaunchEvent::HeaderCreateFolder,
                        ))
                    },
                }
            },
            sidebar_workspace::SidebarWorkspaceEvent::WorkspaceCursorMoved {
                position,
            } => {
                self.state.update_cursor(position);
                Task::none()
            },
            sidebar_workspace::SidebarWorkspaceEvent::QuickLaunch(event) => {
                Task::done(AppEvent::QuickLaunch(event))
            },
            sidebar_workspace::SidebarWorkspaceEvent::Explorer(event) => {
                Task::done(AppEvent::Explorer(event))
            },
        }
    }
}

/// Maximum workspace width ratio, leaving room for minimum tab content.
fn max_sidebar_workspace_ratio() -> f32 {
    (1.0 - SIDEBAR_MIN_TAB_CONTENT_RATIO).max(0.0)
}

#[cfg(test)]
mod tests {
    use iced::Point;

    use super::SidebarFeature;
    use crate::features::sidebar::{SidebarEvent, SidebarItem};
    use crate::ui::widgets::{sidebar_menu, sidebar_workspace};

    #[test]
    fn given_toggle_visibility_event_when_reduced_then_sidebar_hidden_state_toggles()
     {
        let mut feature = SidebarFeature::new();

        let _task = feature.reduce(SidebarEvent::ToggleVisibility, &());

        assert!(feature.is_hidden());
    }

    #[test]
    fn given_select_explorer_event_when_reduced_then_active_item_changes() {
        let mut feature = SidebarFeature::new();

        let _task = feature.reduce(
            SidebarEvent::Menu(sidebar_menu::SidebarMenuEvent::SelectItem(
                sidebar_menu::SidebarMenuItem::Explorer,
            )),
            &(),
        );

        assert_eq!(feature.active_item(), SidebarItem::Explorer);
        assert!(feature.is_workspace_open());
    }

    #[test]
    fn given_add_menu_open_and_dismiss_events_when_reduced_then_overlay_state_changes()
     {
        let mut feature = SidebarFeature::new();

        let _open_task = feature.reduce(
            SidebarEvent::Workspace(
                sidebar_workspace::SidebarWorkspaceEvent::TerminalAddMenuOpen,
            ),
            &(),
        );
        assert!(feature.has_add_menu_open());

        let _dismiss_task = feature.reduce(
            SidebarEvent::Workspace(
                sidebar_workspace::SidebarWorkspaceEvent::TerminalAddMenuDismiss,
            ),
            &(),
        );
        assert!(!feature.has_add_menu_open());
    }

    #[test]
    fn given_cursor_move_event_when_reduced_then_cursor_snapshot_is_updated() {
        let mut feature = SidebarFeature::new();
        let expected = Point::new(42.0, 24.0);

        let _task = feature.reduce(
            SidebarEvent::PaneGridCursorMoved { position: expected },
            &(),
        );

        assert_eq!(feature.cursor(), expected);
    }
}

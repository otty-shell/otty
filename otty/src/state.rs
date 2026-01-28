use std::collections::BTreeMap;

use iced::Size;
use iced::widget::pane_grid;

use crate::features::tab::{TabContent, TabItem};
use crate::ui::widgets::tab_bar;

/// Fixed width of the sidebar menu rail.
pub(crate) const SIDEBAR_MENU_WIDTH: f32 = 52.0;
/// Default workspace width ratio within the sidebar split region.
pub(crate) const SIDEBAR_DEFAULT_WORKSPACE_RATIO: f32 = 0.2;
/// Collapse threshold ratio for the workspace within the split region.
pub(crate) const SIDEBAR_COLLAPSE_WORKSPACE_RATIO: f32 =
    SIDEBAR_DEFAULT_WORKSPACE_RATIO * 0.2;
/// Minimum width ratio reserved for tab content.
pub(crate) const SIDEBAR_MIN_TAB_CONTENT_RATIO: f32 = 0.2;

/// Sidebar menu destinations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SidebarItem {
    Terminal,
}

/// Pane slots in the sidebar split view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SidebarPane {
    Workspace,
    Content,
}

/// Sidebar layout state shared across the app.
pub(crate) struct SidebarState {
    pub(crate) active_item: SidebarItem,
    pub(crate) workspace_open: bool,
    pub(crate) workspace_ratio: f32,
    pub(crate) split: pane_grid::Split,
    pub(crate) panes: pane_grid::State<SidebarPane>,
}

impl SidebarState {
    /// Build a default sidebar layout state.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Workspace ratio used for sizing when the workspace is visible.
    pub(crate) fn effective_workspace_ratio(&self) -> f32 {
        if self.workspace_open {
            self.workspace_ratio
        } else {
            0.0
        }
    }
}

impl Default for SidebarState {
    fn default() -> Self {
        let (mut panes, workspace_pane) =
            pane_grid::State::new(SidebarPane::Workspace);
        let (_content_pane, split) = panes
            .split(
                pane_grid::Axis::Vertical,
                workspace_pane,
                SidebarPane::Content,
            )
            .expect("sidebar split failed");

        panes.resize(split, SIDEBAR_DEFAULT_WORKSPACE_RATIO);

        Self {
            active_item: SidebarItem::Terminal,
            workspace_open: true,
            workspace_ratio: SIDEBAR_DEFAULT_WORKSPACE_RATIO,
            split,
            panes,
        }
    }
}

#[derive(Default)]
pub(crate) struct State {
    pub(crate) active_tab_id: Option<u64>,
    pub(crate) tab_items: BTreeMap<u64, TabItem>,
    pub(crate) next_tab_id: u64,
    pub(crate) next_terminal_id: u64,
    pub(crate) window_size: Size,
    pub(crate) screen_size: Size,
    pub(crate) sidebar: SidebarState,
}

impl State {
    pub(crate) fn new(window_size: Size, screen_size: Size) -> Self {
        Self {
            window_size,
            screen_size,
            sidebar: SidebarState::new(),
            ..Default::default()
        }
    }

    pub(crate) fn active_tab_title(&self) -> Option<&str> {
        self.active_tab_id
            .and_then(|id| self.tab_items.get(&id))
            .map(|tab| tab.title.as_str())
    }

    pub(crate) fn tab_summaries(&self) -> Vec<(u64, String)> {
        self.tab_items
            .iter()
            .map(|(id, item)| (*id, item.title.clone()))
            .collect()
    }

    pub(crate) fn active_tab(&self) -> Option<&TabItem> {
        let id = self.active_tab_id?;
        self.tab_items.get(&id)
    }

    pub(crate) fn set_screen_size(&mut self, size: Size) {
        self.screen_size = size;
        self.sync_tab_grid_sizes();
    }

    pub(crate) fn sync_tab_grid_sizes(&mut self) {
        let size = self.pane_grid_size();
        for tab in self.tab_items.values_mut() {
            if let TabContent::Terminal(terminal) = &mut tab.content {
                terminal.set_grid_size(size);
            }
        }
    }

    pub(crate) fn pane_grid_size(&self) -> Size {
        let tab_bar_height = tab_bar::TAB_BAR_HEIGHT;
        let height = (self.screen_size.height - tab_bar_height).max(0.0);
        let available_width =
            (self.screen_size.width - SIDEBAR_MENU_WIDTH).max(0.0);
        let workspace_ratio = self.sidebar.effective_workspace_ratio();
        let width = (available_width * (1.0 - workspace_ratio)).max(0.0);
        Size::new(width, height)
    }
}

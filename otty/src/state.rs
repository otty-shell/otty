use std::collections::{BTreeMap, HashMap};
use std::time::{Duration, Instant};

use iced::widget::pane_grid;
use iced::{Point, Size};

use crate::features::explorer::state::ExplorerState;
use crate::features::quick_launches::state::QuickLaunchState;
use crate::features::settings::SettingsState;
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
const SIDEBAR_RESIZE_HOVER_SUPPRESS_MS: u64 = 200;

/// Sidebar menu destinations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SidebarItem {
    Terminal,
    Explorer,
}

/// Pane slots in the sidebar split view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SidebarPane {
    Workspace,
    Content,
}

/// Sidebar layout state shared across the app.
#[derive(Debug)]
pub(crate) struct SidebarState {
    pub(crate) active_item: SidebarItem,
    pub(crate) hidden: bool,
    pub(crate) workspace_open: bool,
    pub(crate) workspace_ratio: f32,
    pub(crate) split: pane_grid::Split,
    pub(crate) panes: pane_grid::State<SidebarPane>,
    pub(crate) add_menu: Option<SidebarAddMenuState>,
    pub(crate) cursor: Point,
    last_resize_at: Option<Instant>,
}

impl SidebarState {
    /// Build a default sidebar layout state.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Workspace ratio used for sizing when the workspace is visible.
    pub(crate) fn effective_workspace_ratio(&self) -> f32 {
        if self.hidden {
            0.0
        } else if self.workspace_open {
            self.workspace_ratio
        } else {
            0.0
        }
    }

    pub(crate) fn mark_resizing(&mut self) {
        self.last_resize_at = Some(Instant::now());
    }

    pub(crate) fn is_resizing(&self) -> bool {
        self.last_resize_at
            .map(|last| {
                last.elapsed()
                    <= Duration::from_millis(SIDEBAR_RESIZE_HOVER_SUPPRESS_MS)
            })
            .unwrap_or(false)
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
            hidden: false,
            workspace_open: true,
            workspace_ratio: SIDEBAR_DEFAULT_WORKSPACE_RATIO,
            split,
            panes,
            add_menu: None,
            cursor: Point::ORIGIN,
            last_resize_at: None,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SidebarAddMenuState {
    pub(crate) cursor: Point,
}

#[derive(Default)]
pub(crate) struct State {
    pub(crate) active_tab_id: Option<u64>,
    pub(crate) tab_items: BTreeMap<u64, TabItem>,
    pub(crate) terminal_to_tab: HashMap<u64, u64>,
    pub(crate) next_tab_id: u64,
    pub(crate) next_terminal_id: u64,
    pub(crate) window_size: Size,
    pub(crate) screen_size: Size,
    pub(crate) sidebar: SidebarState,
    pub(crate) quick_launches: QuickLaunchState,
    pub(crate) explorer: ExplorerState,
    pub(crate) settings: SettingsState,
}

impl State {
    pub(crate) fn new(
        window_size: Size,
        screen_size: Size,
        settings: SettingsState,
    ) -> Self {
        let quick_launches = QuickLaunchState::load();
        Self {
            window_size,
            screen_size,
            sidebar: SidebarState::new(),
            quick_launches,
            explorer: ExplorerState::new(),
            settings,
            ..Default::default()
        }
    }

    pub(crate) fn active_tab_title(&self) -> Option<&str> {
        self.active_tab_id
            .and_then(|id| self.tab_items.get(&id))
            .map(|tab| tab.title.as_str())
    }

    pub(crate) fn tab_summaries(&self) -> Vec<(u64, &str)> {
        self.tab_items
            .iter()
            .map(|(id, item)| (*id, item.title.as_str()))
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

    pub(crate) fn register_terminal_for_tab(
        &mut self,
        terminal_id: u64,
        tab_id: u64,
    ) {
        self.terminal_to_tab.insert(terminal_id, tab_id);
    }

    pub(crate) fn remove_tab_terminals(&mut self, tab_id: u64) {
        self.terminal_to_tab
            .retain(|_, mapped_tab| *mapped_tab != tab_id);
    }

    pub(crate) fn terminal_tab_id(&self, terminal_id: u64) -> Option<u64> {
        self.terminal_to_tab.get(&terminal_id).copied()
    }

    pub(crate) fn reindex_terminal_tabs(&mut self) {
        self.terminal_to_tab.clear();
        for (&tab_id, tab) in &self.tab_items {
            if let TabContent::Terminal(terminal) = &tab.content {
                for terminal_id in terminal.terminals().keys().copied() {
                    self.terminal_to_tab.insert(terminal_id, tab_id);
                }
            }
        }
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
        let menu_width = if self.sidebar.hidden {
            0.0
        } else {
            SIDEBAR_MENU_WIDTH
        };
        let available_width = (self.screen_size.width - menu_width).max(0.0);
        let workspace_ratio = self.sidebar.effective_workspace_ratio();
        let width = (available_width * (1.0 - workspace_ratio)).max(0.0);
        Size::new(width, height)
    }
}

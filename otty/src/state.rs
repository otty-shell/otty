use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use iced::widget::pane_grid;
use iced::{Point, Size};

use crate::tab::TabItem;
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
    active_item: SidebarItem,
    hidden: bool,
    workspace_open: bool,
    workspace_ratio: f32,
    split: pane_grid::Split,
    panes: pane_grid::State<SidebarPane>,
    add_menu: Option<SidebarAddMenuState>,
    cursor: Point,
    last_resize_at: Option<Instant>,
}

impl SidebarState {
    /// Build a default sidebar layout state.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    // ── Accessors ──────────────────────────────────────────────────────────

    /// Return the currently active sidebar menu item.
    pub(crate) fn active_item(&self) -> SidebarItem {
        self.active_item
    }

    /// Return whether the sidebar is hidden.
    pub(crate) fn is_hidden(&self) -> bool {
        self.hidden
    }

    /// Return whether the workspace panel is open.
    pub(crate) fn is_workspace_open(&self) -> bool {
        self.workspace_open
    }

    /// Return read-only access to the pane grid state.
    pub(crate) fn panes(&self) -> &pane_grid::State<SidebarPane> {
        &self.panes
    }

    /// Return the current cursor position snapshot.
    pub(crate) fn cursor(&self) -> Point {
        self.cursor
    }

    /// Return the add-menu state if the menu is open.
    pub(crate) fn add_menu(&self) -> Option<&SidebarAddMenuState> {
        self.add_menu.as_ref()
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

    // ── Mutators ───────────────────────────────────────────────────────────

    /// Toggle sidebar visibility between hidden and visible.
    pub(crate) fn toggle_visibility(&mut self) {
        self.hidden = !self.hidden;
    }

    /// Set the active sidebar menu item.
    pub(crate) fn set_active_item(&mut self, item: SidebarItem) {
        self.active_item = item;
    }

    /// Update the sidebar cursor position snapshot.
    pub(crate) fn update_cursor(&mut self, position: Point) {
        self.cursor = position;
    }

    /// Open the add-menu anchored at the current cursor position.
    pub(crate) fn open_add_menu(&mut self) {
        self.add_menu = Some(SidebarAddMenuState {
            cursor: self.cursor,
        });
    }

    /// Close and clear the add-menu.
    pub(crate) fn dismiss_add_menu(&mut self) {
        self.add_menu = None;
    }

    // ── Domain operations ──────────────────────────────────────────────────

    /// Open the workspace at the given ratio, storing it for future use.
    fn open_workspace(&mut self, ratio: f32) {
        self.workspace_open = true;
        self.workspace_ratio = ratio;
        self.panes.resize(self.split, ratio);
    }

    /// Collapse the workspace panel to zero width.
    fn close_workspace(&mut self) {
        self.workspace_open = false;
        self.panes.resize(self.split, 0.0);
    }

    /// Toggle the workspace open or closed.
    ///
    /// Returns `true` when the layout changed and terminal grids must be
    /// re-synced by the caller.
    pub(crate) fn toggle_workspace(&mut self, max_ratio: f32) -> bool {
        if self.workspace_open {
            self.close_workspace();
        } else {
            let ratio = self
                .workspace_ratio
                .max(SIDEBAR_DEFAULT_WORKSPACE_RATIO)
                .min(max_ratio);
            self.open_workspace(ratio);
        }
        true
    }

    /// Ensure the workspace is open, opening it at the default ratio if not.
    ///
    /// Returns `true` when the panel was opened and terminal grids must be
    /// re-synced by the caller; `false` if it was already open.
    pub(crate) fn ensure_workspace_open(&mut self, max_ratio: f32) -> bool {
        if self.workspace_open {
            return false;
        }
        let ratio = self
            .workspace_ratio
            .max(SIDEBAR_DEFAULT_WORKSPACE_RATIO)
            .min(max_ratio);
        self.open_workspace(ratio);
        true
    }

    /// Apply a pane-grid resize event, clamping to `max_ratio` and
    /// auto-collapsing when below the collapse threshold.
    ///
    /// Returns `true` when the layout changed and terminal grids must be
    /// re-synced by the caller.
    pub(crate) fn apply_resize(
        &mut self,
        event: pane_grid::ResizeEvent,
        max_ratio: f32,
    ) -> bool {
        let collapse_threshold = SIDEBAR_COLLAPSE_WORKSPACE_RATIO;

        if !self.workspace_open {
            if event.ratio <= collapse_threshold {
                self.panes.resize(self.split, 0.0);
                return false;
            }
            let ratio = collapse_threshold.min(max_ratio);
            self.open_workspace(ratio);
            return true;
        }

        if event.ratio <= collapse_threshold {
            self.close_workspace();
            return true;
        }

        let ratio = event.ratio.min(max_ratio);
        self.workspace_ratio = ratio;
        self.panes.resize(self.split, ratio);
        true
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

/// Runtime state for workspace tabs.
#[derive(Default)]
pub(crate) struct TabState {
    active_tab_id: Option<u64>,
    tab_items: BTreeMap<u64, TabItem>,
    next_tab_id: u64,
}

impl TabState {
    /// Return active tab identifier.
    pub(crate) fn active_tab_id(&self) -> Option<u64> {
        self.active_tab_id
    }

    /// Return all tab items keyed by tab identifier.
    pub(crate) fn tab_items(&self) -> &BTreeMap<u64, TabItem> {
        &self.tab_items
    }

    /// Return mutable tab item by identifier.
    pub(crate) fn tab_item_mut(&mut self, tab_id: u64) -> Option<&mut TabItem> {
        self.tab_items.get_mut(&tab_id)
    }

    /// Return number of tabs.
    pub(crate) fn len(&self) -> usize {
        self.tab_items.len()
    }

    /// Return whether there are no tabs.
    pub(crate) fn is_empty(&self) -> bool {
        self.tab_items.is_empty()
    }

    /// Return active tab item if present.
    pub(crate) fn active_tab(&self) -> Option<&TabItem> {
        let tab_id = self.active_tab_id?;
        self.tab_items.get(&tab_id)
    }

    /// Check whether tab with the provided identifier exists.
    pub(crate) fn contains(&self, tab_id: u64) -> bool {
        self.tab_items.contains_key(&tab_id)
    }

    /// Allocate next unique tab identifier.
    pub(crate) fn allocate_tab_id(&mut self) -> u64 {
        let tab_id = self.next_tab_id;
        self.next_tab_id += 1;
        tab_id
    }

    /// Insert tab metadata by identifier.
    pub(crate) fn insert(&mut self, tab_id: u64, item: TabItem) {
        self.tab_items.insert(tab_id, item);
    }

    /// Remove tab metadata by identifier.
    pub(crate) fn remove(&mut self, tab_id: u64) -> Option<TabItem> {
        self.tab_items.remove(&tab_id)
    }

    /// Activate tab identifier.
    pub(crate) fn activate(&mut self, tab_id: Option<u64>) {
        self.active_tab_id = tab_id;
    }

    /// Update title for an existing tab.
    pub(crate) fn set_title(&mut self, tab_id: u64, title: String) {
        if let Some(tab) = self.tab_item_mut(tab_id) {
            tab.set_title(title);
        }
    }

    /// Return previous tab identifier before `tab_id`.
    pub(crate) fn previous_tab_id(&self, tab_id: u64) -> Option<u64> {
        self.tab_items
            .range(..tab_id)
            .next_back()
            .map(|(&id, _)| id)
    }

    /// Return last tab identifier in order.
    pub(crate) fn last_tab_id(&self) -> Option<u64> {
        self.tab_items.keys().next_back().copied()
    }
}

#[derive(Default)]
pub(crate) struct State {
    pub(crate) tab: TabState,
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

    pub(crate) fn active_tab_id(&self) -> Option<u64> {
        self.tab.active_tab_id()
    }

    pub(crate) fn allocate_tab_id(&mut self) -> u64 {
        self.tab.allocate_tab_id()
    }

    pub(crate) fn active_tab_title(&self) -> Option<&str> {
        self.tab.active_tab().map(|tab| tab.title())
    }

    pub(crate) fn tab_summaries(&self) -> Vec<(u64, &str)> {
        self.tab
            .tab_items()
            .iter()
            .map(|(id, item)| (*id, item.title()))
            .collect()
    }

    pub(crate) fn active_tab(&self) -> Option<&TabItem> {
        self.tab.active_tab()
    }

    pub(crate) fn set_screen_size(&mut self, size: Size) {
        self.screen_size = size;
    }

    /// Compute available pane grid size from current screen and sidebar layout.
    pub(crate) fn pane_grid_size(&self) -> Size {
        let tab_bar_height = tab_bar::TAB_BAR_HEIGHT;
        let height = (self.screen_size.height - tab_bar_height).max(0.0);
        let menu_width = if self.sidebar.is_hidden() {
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

/// Maximum workspace width ratio, leaving room for the minimum tab content.
pub(crate) fn max_sidebar_workspace_ratio() -> f32 {
    (1.0 - SIDEBAR_MIN_TAB_CONTENT_RATIO).max(0.0)
}

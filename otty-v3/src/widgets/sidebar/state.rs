use std::time::{Duration, Instant};

use iced::Point;
use iced::widget::pane_grid;

use super::model::{SidebarItem, SidebarPane};

const SIDEBAR_DEFAULT_WORKSPACE_RATIO: f32 = 0.2;
const SIDEBAR_COLLAPSE_WORKSPACE_RATIO: f32 =
    SIDEBAR_DEFAULT_WORKSPACE_RATIO * 0.2;
const SIDEBAR_RESIZE_HOVER_SUPPRESS_MS: u64 = 200;

/// Internal runtime state for sidebar layout and interaction metadata.
#[derive(Debug)]
pub(super) struct SidebarState {
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
    pub(super) fn active_item(&self) -> SidebarItem {
        self.active_item
    }

    pub(super) fn is_hidden(&self) -> bool {
        self.hidden
    }

    pub(super) fn is_workspace_open(&self) -> bool {
        self.workspace_open
    }

    pub(super) fn panes(&self) -> &pane_grid::State<SidebarPane> {
        &self.panes
    }

    pub(super) fn cursor(&self) -> Point {
        self.cursor
    }

    pub(super) fn add_menu_cursor(&self) -> Option<Point> {
        self.add_menu.as_ref().map(|menu| menu.cursor)
    }

    pub(super) fn has_add_menu_open(&self) -> bool {
        self.add_menu.is_some()
    }

    pub(super) fn effective_workspace_ratio(&self) -> f32 {
        if self.hidden {
            0.0
        } else if self.workspace_open {
            self.workspace_ratio
        } else {
            0.0
        }
    }

    pub(super) fn is_resizing(&self) -> bool {
        self.last_resize_at
            .map(|last| {
                last.elapsed()
                    <= Duration::from_millis(SIDEBAR_RESIZE_HOVER_SUPPRESS_MS)
            })
            .unwrap_or(false)
    }

    pub(super) fn toggle_visibility(&mut self) {
        self.hidden = !self.hidden;
    }

    pub(super) fn set_active_item(&mut self, item: SidebarItem) {
        self.active_item = item;
    }

    pub(super) fn update_cursor(&mut self, position: Point) {
        self.cursor = position;
    }

    pub(super) fn open_add_menu(&mut self) {
        self.add_menu = Some(SidebarAddMenuState {
            cursor: self.cursor,
        });
    }

    pub(super) fn dismiss_add_menu(&mut self) {
        self.add_menu = None;
    }

    pub(super) fn mark_resizing(&mut self) {
        self.last_resize_at = Some(Instant::now());
    }

    pub(super) fn toggle_workspace(&mut self, max_ratio: f32) -> bool {
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

    pub(super) fn ensure_workspace_open(&mut self, max_ratio: f32) -> bool {
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

    pub(super) fn apply_resize(
        &mut self,
        event: pane_grid::ResizeEvent,
        max_ratio: f32,
    ) -> bool {
        if !self.workspace_open {
            if event.ratio <= SIDEBAR_COLLAPSE_WORKSPACE_RATIO {
                self.panes.resize(self.split, 0.0);
                return false;
            }
            let ratio = SIDEBAR_COLLAPSE_WORKSPACE_RATIO.min(max_ratio);
            self.open_workspace(ratio);
            return true;
        }

        if event.ratio <= SIDEBAR_COLLAPSE_WORKSPACE_RATIO {
            self.close_workspace();
            return true;
        }

        let ratio = event.ratio.min(max_ratio);
        self.workspace_ratio = ratio;
        self.panes.resize(self.split, ratio);
        true
    }

    fn open_workspace(&mut self, ratio: f32) {
        self.workspace_open = true;
        self.workspace_ratio = ratio;
        self.panes.resize(self.split, ratio);
    }

    fn close_workspace(&mut self) {
        self.workspace_open = false;
        self.panes.resize(self.split, 0.0);
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

#[derive(Debug, Clone, Copy)]
struct SidebarAddMenuState {
    cursor: Point,
}

use iced::Size;

/// Shared compact control size used by dense toolbars and menus.
pub(crate) const BUTTON_SIZE_COMPACT: f32 = 24.0;
/// Shared regular control size used by form actions.
pub(crate) const BUTTON_SIZE_REGULAR: f32 = 28.0;
/// Shared large control size used by sidebar rail actions.
pub(crate) const BUTTON_SIZE_RAIL: f32 = 44.0;
/// Shared rounded corner radius for standard buttons.
pub(crate) const BUTTON_RADIUS_ROUNDED: f32 = 6.0;

// ============================================================
// Modern UI 分级圆角 token（精确对齐 VS Code styleOverrides 范式）：
//   CONTROL(4px) = 可交互控件：按钮/输入/列表行/标签页
//   INNER(6px)   = 内嵌容器：侧边栏/面板
//   OUTER(8px)   = 悬浮层 + 编辑器主角卡片
// ============================================================
pub(crate) const RADIUS_CONTROL: f32 = 4.0;
pub(crate) const RADIUS_INNER: f32 = 6.0;
pub(crate) const RADIUS_OUTER: f32 = 8.0;

/// Outer spacing between framed workspace surfaces and the window edge.
pub(crate) const SURFACE_OUTER_MARGIN: f32 = 4.0;
/// Spacing between adjacent workspace surfaces.
pub(crate) const SURFACE_GAP: f32 = 4.0;
/// Border width of framed workspace surfaces.
pub(crate) const SURFACE_BORDER: f32 = 1.0;
/// Total height of the editor tab bar area.
pub(crate) const TAB_BAR_HEIGHT: f32 = 28.0;
/// Minimum usable editor width before workspace ratio clamping.
pub(crate) const MIN_EDITOR_WIDTH: f32 = 320.0;
/// Fixed width of the sidebar menu rail.
pub(crate) const SIDEBAR_RAIL_WIDTH: f32 = 52.0;

pub(crate) fn screen_size_from_window(window_size: Size) -> Size {
    Size::new(window_size.width, window_size.height)
}

/// Pure workspace surface geometry shared by rendering and terminal sizing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct WorkspaceGeometry {
    pub(crate) outer_width: f32,
    pub(crate) outer_height: f32,
    pub(crate) rail_width: f32,
    pub(crate) pane_grid_width: f32,
    pub(crate) sidebar_width: f32,
    pub(crate) editor_width: f32,
    pub(crate) editor_height: f32,
    pub(crate) terminal_grid_size: Size,
}

impl WorkspaceGeometry {
    pub(crate) fn new(
        screen_size: Size,
        sidebar_visible: bool,
        workspace_open: bool,
        workspace_ratio: f32,
    ) -> Self {
        let outer_width =
            (screen_size.width - SURFACE_OUTER_MARGIN * 2.0).max(0.0);
        let outer_height =
            (screen_size.height - SURFACE_OUTER_MARGIN * 2.0).max(0.0);
        let rail_width = if sidebar_visible {
            SIDEBAR_RAIL_WIDTH
        } else {
            0.0
        };
        let workspace_open = workspace_open && sidebar_visible;
        let pane_grid_width = if sidebar_visible {
            (outer_width - rail_width - SURFACE_GAP).max(0.0)
        } else {
            outer_width
        };
        let split_width = (pane_grid_width - SURFACE_GAP).max(0.0);
        let max_ratio = if split_width > 0.0 {
            (1.0 - MIN_EDITOR_WIDTH / split_width).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let ratio = if workspace_open {
            workspace_ratio.clamp(0.0, max_ratio)
        } else {
            0.0
        };
        let sidebar_width = split_width * ratio;
        let editor_width = if sidebar_visible {
            split_width * (1.0 - ratio)
        } else {
            pane_grid_width
        };
        let editor_height = outer_height;
        let terminal_grid_size = Size::new(
            (editor_width - SURFACE_BORDER * 2.0).max(0.0),
            (editor_height - SURFACE_BORDER * 2.0 - TAB_BAR_HEIGHT).max(0.0),
        );

        Self {
            outer_width,
            outer_height,
            rail_width,
            pane_grid_width,
            sidebar_width,
            editor_width,
            editor_height,
            terminal_grid_size,
        }
    }

    /// Return the terminal pane grid size for the current workspace layout.
    pub(crate) fn pane_grid_size(&self) -> Size {
        self.terminal_grid_size
    }
}

#[cfg(test)]
mod tests {
    use iced::Size;

    use super::{
        MIN_EDITOR_WIDTH, SURFACE_BORDER, SURFACE_GAP, SURFACE_OUTER_MARGIN,
        TAB_BAR_HEIGHT, WorkspaceGeometry,
    };

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 0.001,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn given_sidebar_visible_when_geometry_computed_then_matches_surface_metrics()
     {
        let geometry =
            WorkspaceGeometry::new(Size::new(1280.0, 800.0), true, true, 0.3);

        assert_close(geometry.outer_width, 1272.0);
        assert_close(geometry.outer_height, 792.0);
        assert_close(geometry.rail_width, 52.0);
        assert_close(geometry.pane_grid_width, 1216.0);
        assert_close(geometry.sidebar_width, 363.6);
        assert_close(geometry.editor_width, 848.4);
        assert_close(
            geometry.terminal_grid_size.width,
            geometry.editor_width - SURFACE_BORDER * 2.0,
        );
        assert_close(
            geometry.terminal_grid_size.height,
            geometry.outer_height - SURFACE_BORDER * 2.0 - TAB_BAR_HEIGHT,
        );
    }

    #[test]
    fn given_sidebar_hidden_when_geometry_computed_then_editor_fills_surface() {
        let geometry =
            WorkspaceGeometry::new(Size::new(800.0, 600.0), false, true, 0.5);

        assert_close(geometry.rail_width, 0.0);
        assert_close(geometry.pane_grid_width, 792.0);
        assert_close(geometry.sidebar_width, 0.0);
        assert_close(geometry.editor_width, 792.0);
        assert_close(
            geometry.terminal_grid_size.height,
            600.0
                - SURFACE_OUTER_MARGIN * 2.0
                - SURFACE_BORDER * 2.0
                - TAB_BAR_HEIGHT,
        );
    }

    #[test]
    fn given_workspace_closed_when_geometry_computed_then_editor_keeps_content_pane_width()
     {
        let geometry =
            WorkspaceGeometry::new(Size::new(1280.0, 800.0), true, false, 0.6);

        assert_close(geometry.sidebar_width, 0.0);
        assert_close(
            geometry.editor_width,
            geometry.pane_grid_width - SURFACE_GAP,
        );
        assert_close(
            geometry.terminal_grid_size.width,
            geometry.pane_grid_width - SURFACE_GAP - SURFACE_BORDER * 2.0,
        );
    }

    #[test]
    fn given_sidebar_hidden_when_geometry_computed_then_editor_is_full_pane() {
        let geometry =
            WorkspaceGeometry::new(Size::new(800.0, 600.0), false, false, 0.3);

        assert_close(geometry.pane_grid_width, 792.0);
        assert_close(geometry.editor_width, geometry.pane_grid_width);
        assert_close(
            geometry.terminal_grid_size.width,
            geometry.pane_grid_width - SURFACE_BORDER * 2.0,
        );
    }

    #[test]
    fn given_oversized_workspace_ratio_when_geometry_computed_then_editor_keeps_minimum_width()
     {
        let geometry =
            WorkspaceGeometry::new(Size::new(800.0, 600.0), true, true, 0.9);

        assert!(geometry.editor_width >= MIN_EDITOR_WIDTH);
        assert_close(geometry.editor_width, MIN_EDITOR_WIDTH);
    }

    #[test]
    fn given_tiny_window_when_geometry_computed_then_sizes_are_zero_safe() {
        let geometry =
            WorkspaceGeometry::new(Size::new(10.0, 10.0), true, true, 0.5);

        assert!(geometry.terminal_grid_size.width >= 0.0);
        assert!(geometry.terminal_grid_size.height >= 0.0);
    }

    #[test]
    fn pane_grid_size_matches_geometry_width_and_height() {
        let geometry =
            WorkspaceGeometry::new(Size::new(1280.0, 800.0), true, true, 0.3);
        let size = geometry.pane_grid_size();

        assert_close(size.width, geometry.editor_width - SURFACE_BORDER * 2.0);
        assert_close(
            size.height,
            geometry.outer_height - SURFACE_BORDER * 2.0 - TAB_BAR_HEIGHT,
        );
    }
}

use iced::Size;

use crate::widgets::tabs::view::tab_bar::TAB_BAR_HEIGHT;

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

pub(crate) fn screen_size_from_window(window_size: Size) -> Size {
    Size::new(window_size.width, window_size.height)
}

pub(crate) fn pane_grid_size(
    screen_size: Size,
    sidebar_is_hidden: bool,
    sidebar_menu_width: f32,
    sidebar_workspace_ratio: f32,
) -> Size {
    let height = (screen_size.height - TAB_BAR_HEIGHT).max(0.0);

    let menu_width = if sidebar_is_hidden {
        0.0
    } else {
        sidebar_menu_width
    };

    let available_width = (screen_size.width - menu_width).max(0.0);
    let width = (available_width * (1.0 - sidebar_workspace_ratio)).max(0.0);

    Size::new(width, height)
}

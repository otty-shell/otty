use iced::Background;
use iced::widget::{container, scrollable};

use super::theme::{ThemeProps, UiColorPalette, mix_color};
use crate::layout::{RADIUS_CONTROL, RADIUS_OUTER};

pub(crate) fn thin_scroll_style(
    palette: UiColorPalette,
) -> impl Fn(&iced::Theme, scrollable::Status) -> scrollable::Style + 'static {
    move |theme, status| {
        let mut style = scrollable::default(theme, status);
        // 滚动条滑块：控件级圆角（VS Code 范式 control=4px）
        let radius = iced::border::Radius::from(RADIUS_CONTROL);

        style.vertical_rail.border.radius = radius;
        style.vertical_rail.scroller.border.radius = radius;
        style.horizontal_rail.border.radius = radius;
        style.horizontal_rail.scroller.border.radius = radius;

        let mut scroller_color = match style.vertical_rail.scroller.background {
            Background::Color(color) => color,
            _ => palette.muted_foreground,
        };
        scroller_color.a = (scroller_color.a * 0.7).min(1.0);
        style.vertical_rail.scroller.background =
            Background::Color(scroller_color);
        style.horizontal_rail.scroller.background =
            Background::Color(scroller_color);

        style
    }
}

pub(crate) fn tree_row_style(
    palette: &UiColorPalette,
    is_selected: bool,
    is_hovered: bool,
) -> container::Style {
    // 自适应混合（color-mix 等价）：选中=玫瑰红~35%（对应 VS Code list.activeSelectionBackground），hover=前景 8%
    let background = if is_selected {
        Some(mix_color(palette.accent, palette.surface_background, 0.35).into())
    } else if is_hovered {
        Some(
            mix_color(palette.foreground, palette.surface_background, 0.08)
                .into(),
        )
    } else {
        None
    };

    container::Style {
        background,
        text_color: Some(palette.foreground),
        ..Default::default()
    }
}

pub(crate) fn menu_panel_style(
    theme: ThemeProps<'_>,
) -> impl Fn(&iced::Theme) -> container::Style + 'static {
    let palette = theme.theme.ui_palette().clone();
    move |_theme: &iced::Theme| container::Style {
        background: Some(palette.overlay.into()),
        text_color: Some(palette.foreground),
        border: iced::Border {
            width: 0.25,
            color: palette.overlay,
            // 菜单属于悬浮层：外层圆角 8px（VS Code 范式 outer=8px）
            radius: iced::border::Radius::new(RADIUS_OUTER),
        },
        ..Default::default()
    }
}

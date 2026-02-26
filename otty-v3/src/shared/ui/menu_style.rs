use iced::widget::container;

use super::theme::ThemeProps;

/// Return a styled container closure for context menu panels.
pub(crate) fn menu_panel_style(
    theme: ThemeProps<'_>,
) -> impl Fn(&iced::Theme) -> container::Style + 'static {
    let palette = theme.theme.iced_palette().clone();
    move |_theme: &iced::Theme| container::Style {
        background: Some(palette.overlay.into()),
        text_color: Some(palette.foreground),
        border: iced::Border {
            width: 0.25,
            color: palette.overlay,
            radius: iced::border::Radius::new(4.0),
        },
        ..Default::default()
    }
}

use iced::widget::{button, container, svg};
use iced::{Element, Length, alignment};

use crate::shared::ui::theme::{StyleOverrides, ThemeProps};

/// UI events emitted by an icon button.
#[derive(Debug, Clone)]
pub(crate) enum IconButtonEvent {
    Pressed,
}

/// Visual variants for an icon button.
#[derive(Debug, Clone, Copy)]
pub(crate) enum IconButtonVariant {
    Standard,
    Danger,
}

/// Props for rendering an icon button.
#[derive(Debug, Clone, Copy)]
pub(crate) struct IconButtonProps<'a> {
    pub(crate) icon: &'static [u8],
    pub(crate) theme: ThemeProps<'a>,
    pub(crate) size: f32,
    pub(crate) icon_size: f32,
    pub(crate) variant: IconButtonVariant,
}

const ICON_BUTTON_PADDING: f32 = 0.0;

/// Render a square icon button used for window controls.
pub(crate) fn view<'a>(
    props: IconButtonProps<'a>,
) -> Element<'a, IconButtonEvent> {
    let palette = props.theme.theme.iced_palette();
    let (base_color, hover_color) = resolve_variant_colors(
        props.variant,
        palette.dim_foreground,
        palette.blue,
        palette.red,
        props.theme.overrides,
    );

    let icon = svg::Svg::new(svg::Handle::from_memory(props.icon))
        .width(Length::Fixed(props.icon_size))
        .height(Length::Fixed(props.icon_size))
        .style(move |_, status| {
            let color = if matches!(status, svg::Status::Hovered) {
                hover_color
            } else {
                base_color
            };

            svg::Style { color: Some(color) }
        });

    let icon_container = container(icon)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center);

    button(icon_container)
        .on_press(IconButtonEvent::Pressed)
        .padding(ICON_BUTTON_PADDING)
        .width(Length::Fixed(props.size))
        .height(Length::Fixed(props.size))
        .style(|_, _| iced::widget::button::Style::default())
        .into()
}

fn resolve_variant_colors(
    variant: IconButtonVariant,
    default_base: iced::Color,
    accent: iced::Color,
    danger: iced::Color,
    overrides: Option<StyleOverrides>,
) -> (iced::Color, iced::Color) {
    if let Some(overrides) = overrides {
        if let Some(color) = overrides.foreground {
            return (color, color);
        }
    }

    match variant {
        IconButtonVariant::Standard => (default_base, accent),
        IconButtonVariant::Danger => (default_base, danger),
    }
}

#[cfg(test)]
mod tests {
    use super::{IconButtonVariant, resolve_variant_colors};
    use crate::shared::ui::theme::StyleOverrides;

    #[test]
    fn given_standard_variant_when_resolving_without_override_then_hover_uses_accent()
     {
        let default_base = iced::Color::from_rgb(0.1, 0.2, 0.3);
        let accent = iced::Color::from_rgb(0.4, 0.5, 0.6);
        let danger = iced::Color::from_rgb(0.7, 0.8, 0.9);

        let (base, hover) = resolve_variant_colors(
            IconButtonVariant::Standard,
            default_base,
            accent,
            danger,
            None,
        );

        assert_eq!(base, default_base);
        assert_eq!(hover, accent);
    }

    #[test]
    fn given_danger_variant_when_resolving_without_override_then_hover_uses_danger()
     {
        let default_base = iced::Color::from_rgb(0.1, 0.2, 0.3);
        let accent = iced::Color::from_rgb(0.4, 0.5, 0.6);
        let danger = iced::Color::from_rgb(0.7, 0.8, 0.9);

        let (base, hover) = resolve_variant_colors(
            IconButtonVariant::Danger,
            default_base,
            accent,
            danger,
            None,
        );

        assert_eq!(base, default_base);
        assert_eq!(hover, danger);
    }

    #[test]
    fn given_foreground_override_when_resolving_then_override_is_used_for_all_states()
     {
        let default_base = iced::Color::from_rgb(0.1, 0.2, 0.3);
        let accent = iced::Color::from_rgb(0.4, 0.5, 0.6);
        let danger = iced::Color::from_rgb(0.7, 0.8, 0.9);
        let override_color = iced::Color::from_rgb(0.3, 0.2, 0.1);
        let overrides = Some(StyleOverrides {
            background: None,
            foreground: Some(override_color),
            border_radius: None,
        });

        let (base, hover) = resolve_variant_colors(
            IconButtonVariant::Standard,
            default_base,
            accent,
            danger,
            overrides,
        );

        assert_eq!(base, override_color);
        assert_eq!(hover, override_color);
    }
}

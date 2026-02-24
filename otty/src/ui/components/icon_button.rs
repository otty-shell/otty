use iced::widget::{button, container, svg};
use iced::{Element, Length, alignment};

use crate::theme::{StyleOverrides, ThemeProps};

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

/// A square icon button used for window controls.
pub(crate) struct IconButton<'a> {
    props: IconButtonProps<'a>,
    padding: f32,
}

impl<'a> IconButton<'a> {
    pub fn new(props: IconButtonProps<'a>) -> Self {
        Self {
            props,
            padding: 0.0,
        }
    }

    pub fn view(self) -> Element<'a, IconButtonEvent> {
        let palette = self.props.theme.theme.iced_palette();
        let (base_color, hover_color) = resolve_variant_colors(
            self.props.variant,
            palette.dim_foreground,
            palette.blue,
            palette.red,
            self.props.theme.overrides,
        );

        let icon = svg::Svg::new(svg::Handle::from_memory(self.props.icon))
            .width(Length::Fixed(self.props.icon_size))
            .height(Length::Fixed(self.props.icon_size))
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
            .padding(self.padding)
            .width(Length::Fixed(self.props.size))
            .height(Length::Fixed(self.props.size))
            .style(|_, _| iced::widget::button::Style::default())
            .into()
    }
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

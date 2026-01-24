use iced::alignment;
use iced::border::Radius;
use iced::widget::text::Wrapping;
use iced::widget::{Space, button, container, row, stack};
use iced::{
    Alignment, Element, Length,
    widget::{svg, text},
};

use crate::app::theme::{StyleOverrides, ThemeProps};
use crate::helpers::ellipsize;
use crate::icons;

/// UI events emitted by a tab button.
#[derive(Debug, Clone)]
pub(crate) enum TabButtonEvent {
    ActivateTab(u64),
    CloseTab(u64),
}

/// Layout metrics for a tab button.
#[derive(Debug, Clone, Copy)]
struct TabButtonMetrics {
    height: f32,
    width: f32,
    padding: f32,
    label_font_size: f32,
    pill_padding: f32,
    close_icon_size: f32,
    close_button_right_padding: f32,
    close_button_padding: f32,
}

impl Default for TabButtonMetrics {
    fn default() -> Self {
        Self {
            height: 25.0,
            width: 235.0,
            padding: 0.0,
            label_font_size: 13.0,
            pill_padding: 2.0,
            close_icon_size: 25.0,
            close_button_right_padding: 2.0,
            close_button_padding: 0.0,
        }
    }
}

/// Props for rendering a tab button.
#[derive(Debug, Clone)]
pub(crate) struct TabButtonProps<'a> {
    pub(crate) id: u64,
    pub(crate) title: String,
    pub(crate) is_active: bool,
    pub(crate) theme: ThemeProps<'a>,
}

/// A clickable tab pill with close affordance.
pub(crate) struct TabButton<'a> {
    props: TabButtonProps<'a>,
    metrics: TabButtonMetrics,
}

impl<'a> TabButton<'a> {
    pub fn new(props: TabButtonProps<'a>) -> Self {
        Self {
            props,
            metrics: TabButtonMetrics::default(),
        }
    }

    pub fn view(self) -> Element<'a, TabButtonEvent> {
        let palette = self.props.theme.theme.iced_palette();
        let foreground = palette.foreground;
        let dim_foreground = palette.dim_foreground;
        let red = palette.red;
        let background = palette.background;
        let dim_black = palette.dim_black;

        let label = text(ellipsize(&self.props.title))
            .size(self.metrics.label_font_size)
            .width(Length::Fill)
            .height(Length::Shrink)
            .align_y(Alignment::Center)
            .align_x(Alignment::Center)
            .wrapping(Wrapping::None);

        let close_icon = svg::Handle::from_memory(icons::WINDOW_CLOSE);
        let close_svg = svg::Svg::new(close_icon)
            .width(Length::Fixed(self.metrics.close_icon_size))
            .height(Length::Fixed(self.metrics.close_icon_size))
            .style({
                let is_active = self.props.is_active;
                move |_, status| {
                    let color = if status == svg::Status::Hovered {
                        red
                    } else if is_active {
                        foreground
                    } else {
                        dim_foreground
                    };

                    svg::Style { color: Some(color) }
                }
            });

        let close_icon_view = container(close_svg)
            .width(Length::Shrink)
            .height(Length::Fill)
            .align_x(alignment::Horizontal::Right)
            .align_y(alignment::Vertical::Center);

        let close_button = button(close_icon_view)
            .on_press(TabButtonEvent::CloseTab(self.props.id))
            .padding(self.metrics.close_button_padding)
            .height(Length::Fill)
            .style(|_, _| iced::widget::button::Style::default());

        let close_button_row = row![
            Space::new().width(Length::Fill),
            close_button,
            Space::new()
                .width(Length::Fixed(self.metrics.close_button_right_padding))
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .align_y(Alignment::Center);

        let label_container = container(label)
            .align_y(Alignment::Center)
            .height(Length::Fill)
            .width(Length::Fill);

        let pill_content = stack![label_container, close_button_row]
            .height(Length::Fill)
            .width(Length::Fill);

        let pill = container(pill_content)
            .padding(self.metrics.pill_padding)
            .width(Length::Fill)
            .height(Length::Fill)
            .style({
                let overrides = self.props.theme.overrides;
                move |_| {
                    if self.props.is_active {
                        tab_style(background, foreground, overrides)
                    } else {
                        tab_style(dim_black, dim_foreground, overrides)
                    }
                }
            });

        button(pill)
            .on_press(TabButtonEvent::ActivateTab(self.props.id))
            .padding(self.metrics.padding)
            .width(self.metrics.width)
            .height(self.metrics.height)
            .into()
    }
}

fn tab_style(
    background: iced::Color,
    foreground: iced::Color,
    overrides: Option<StyleOverrides>,
) -> iced::widget::container::Style {
    let mut style = iced::widget::container::Style {
        background: Some(background.into()),
        text_color: Some(foreground),
        ..Default::default()
    };

    if let Some(overrides) = overrides {
        if let Some(color) = overrides.background {
            style.background = Some(color.into());
        }
        if let Some(color) = overrides.foreground {
            style.text_color = Some(color);
        }
        if let Some(radius) = overrides.border_radius {
            style.border.radius = Radius::new(radius);
        }
    }

    style
}

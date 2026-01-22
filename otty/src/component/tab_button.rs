use iced::alignment;
use iced::widget::{Space, button, container, row, stack};
use iced::widget::text::Wrapping;
use iced::{Alignment, Length, Element, widget::{svg, text}};

use crate::theme::fallback_theme;
use crate::{icons, theme::{AppTheme, IcedColorPalette}, helpers::ellipsize};

#[derive(Debug, Clone)]
pub enum Event {
    ActivateTab(u64),
    CloseTab(u64),
}

#[derive(Debug, Clone)]
pub struct Metrics {
    height: f32,
    width: f32,
    padding: f32,
    label_font_size: f32,
    pill_padding: f32,
    close_icon_size: f32,
    close_button_right_padding: f32,
    close_button_padding: f32,
}

impl Default for Metrics {
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

#[derive(Debug, Default)]
pub struct TabButton<'a> {
    id: u64,
    title: &'a str,
    is_active: bool,
    theme: Option<&'a AppTheme>,
    metrics: Metrics,
}

impl<'a> TabButton<'a> {
    pub fn new(
        id: u64,
        title: &'a str,
    ) -> Self {
        Self {
            id,
            title,
            ..Default::default()
        }
    }

    pub fn theme(mut self, theme: &'a AppTheme) -> Self {
        self.theme = Some(theme);
        self
    }

    pub fn active(mut self, val: bool) -> Self {
        self.is_active = val;
        self
    }

    pub fn view(self) -> Element<'a, Event> {
        let theme = self.theme.unwrap_or(fallback_theme());

        let label = text(ellipsize(self.title))
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
                let theme = theme.iced_palette().clone();
                move |_, status| {
                    let color = if status == svg::Status::Hovered {
                        theme.red
                    } else if self.is_active {
                        theme.foreground
                    } else {
                        theme.dim_foreground
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
            .on_press(Event::CloseTab(self.id))
            .padding(self.metrics.close_button_padding)
            .height(Length::Fill)
            .style(|_, _| iced::widget::button::Style::default());

        let close_button_row = row![
            Space::new().width(Length::Fill),
            close_button,
            Space::new().width(Length::Fixed(self.metrics.close_button_right_padding))
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
                let theme = theme.iced_palette().clone();
                move |_| {
                    if self.is_active {
                        active_tab_style(&theme)
                    } else {
                        inactive_tab_style(&theme)
                    }
                }
            });

        button(pill)
            .on_press(Event::ActivateTab(self.id))
            .padding(self.metrics.padding)
            .width(self.metrics.width)
            .height(self.metrics.height)
            .into()
    }
}

fn active_tab_style(
    theme: &IcedColorPalette,
) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(theme.background.into()),
        text_color: Some(theme.foreground),
        ..Default::default()
    }
}

fn inactive_tab_style(
    theme: &IcedColorPalette,
) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(theme.dim_black.into()),
        text_color: Some(theme.dim_foreground),
        ..Default::default()
    }
}

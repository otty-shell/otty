use iced::widget::text::Wrapping;
use iced::widget::{Space, button, container, row, stack, svg, text};
use iced::{Alignment, alignment};
use iced::{Element, Length, Theme};

use crate::icons;
use crate::main_window::Event;
use crate::theme::IcedColorPalette;

const TAB_LABEL_FONT_SIZE: f32 = 13.0;
const TAB_HEIGHT: f32 = 25.0;
const TAB_WIDTH: f32 = 235.0;
const CLOSE_ICON_SIZE: f32 = 25.0;
const CLOSE_BUTTON_RIGHT_PADDING: f32 = 2.0;
const DEFAULT_MAX_CHAR_COUNT_BEFORE_ELIPSIZE: usize = 20;

pub fn tab_button<'a>(
    title: &'a str,
    is_active: bool,
    id: u64,
    theme: &IcedColorPalette,
) -> Element<'a, Event, Theme, iced::Renderer> {
    let label = text(ellipsize(title))
        .size(TAB_LABEL_FONT_SIZE)
        .width(Length::Fill)
        .height(Length::Shrink)
        .align_y(Alignment::Center)
        .align_x(Alignment::Center)
        .wrapping(Wrapping::None);

    let close_icon = svg::Handle::from_memory(icons::WINDOW_CLOSE);
    let close_svg = svg::Svg::new(close_icon)
        .width(Length::Fixed(CLOSE_ICON_SIZE))
        .height(Length::Fixed(CLOSE_ICON_SIZE))
        .style({
            let theme = theme.clone();
            move |_, status| {
                let color = if status == svg::Status::Hovered {
                    theme.red
                } else if is_active {
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
        .on_press(Event::CloseTab(id))
        .padding(0)
        .height(Length::Fill)
        .style(|_, _| iced::widget::button::Style::default());

    let close_button_row = row![
        Space::new().width(Length::Fill),
        close_button,
        Space::new().width(Length::Fixed(CLOSE_BUTTON_RIGHT_PADDING))
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
        .padding(2)
        .width(Length::Fill)
        .height(Length::Fill)
        .style({
            let theme = theme.clone();
            move |_| {
                if is_active {
                    active_tab_style(&theme)
                } else {
                    inactive_tab_style(&theme)
                }
            }
        });

    button(pill)
        .on_press(Event::ActivateTab(id))
        .padding(0)
        .width(TAB_WIDTH)
        .height(TAB_HEIGHT)
        .into()
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

fn ellipsize(s: &str) -> String {
    let total = s.chars().count();
    if total <= DEFAULT_MAX_CHAR_COUNT_BEFORE_ELIPSIZE {
        return s.to_owned();
    }

    let keep = DEFAULT_MAX_CHAR_COUNT_BEFORE_ELIPSIZE - 2;
    let tail: String = s
        .chars()
        .rev()
        .take(keep)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    format!("..{}", tail)
}

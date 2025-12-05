use iced::widget::text::Wrapping;
use iced::widget::{button, container, row, svg, text};
use iced::{Alignment, alignment};
use iced::{Element, Length, Theme};

use crate::main_window::Event;

const TAB_HEIGHT: f32 = 25.0;
const CLOSE_ICON_SIZE: f32 = 25.0;

pub fn tab_button<'a>(
    title: &str,
    width: f32,
    is_active: bool,
    id: u64,
    font_size: f32,
) -> Element<'a, Event, Theme, iced::Renderer> {
    let elided = elide_title(title, width);
    let label = text(elided)
        .size(font_size)
        .width(Length::Fill)
        .height(Length::Shrink)
        .align_x(Alignment::Center)
        .wrapping(Wrapping::None);

    let close_icon =
        svg::Handle::from_memory(include_bytes!("../../assets/svg/close.svg"));
    let close_svg = svg::Svg::new(close_icon)
        .width(Length::Fixed(CLOSE_ICON_SIZE))
        .height(Length::Fixed(CLOSE_ICON_SIZE));

    let close_icon_view = container(close_svg)
        .width(Length::Shrink)
        .height(Length::Fill)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center);

    let close_button = button(close_icon_view)
        .on_press(Event::CloseTab(id))
        .padding(0)
        .height(Length::Fill)
        .style(|_, _| iced::widget::button::Style::default());

    let label_container = container(label).padding([0, 4]).width(Length::Fill);

    let pill_content = row![label_container, close_button].width(Length::Fill);

    let pill = container(pill_content)
        .padding([4, 0])
        .width(Length::Fixed(width))
        .height(Length::Fill)
        .style(if is_active {
            active_tab_style
        } else {
            inactive_tab_style
        });

    button(pill)
        .on_press(Event::ActivateTab(id))
        .padding(0)
        .height(Length::Fixed(TAB_HEIGHT))
        .into()
}

fn active_tab_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();

    iced::widget::container::Style {
        background: Some(palette.primary.weak.color.into()),
        text_color: Some(palette.primary.weak.text),
        ..Default::default()
    }
}

fn inactive_tab_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();

    iced::widget::container::Style {
        background: Some(palette.background.weak.color.into()),
        text_color: Some(palette.background.weak.text),
        ..Default::default()
    }
}

fn elide_title(title: &str, max_width: f32) -> String {
    const CLOSE_AND_PADDING: f32 = 32.0;
    const AVG_CHAR_WIDTH: f32 = 7.0;

    let available = (max_width - CLOSE_AND_PADDING).max(0.0);
    let max_chars = (available / AVG_CHAR_WIDTH).floor() as usize;

    if max_chars == 0 {
        return String::from("...");
    }

    let mut chars: Vec<char> = title.chars().collect();
    if chars.len() <= max_chars {
        return title.to_string();
    }

    let keep = max_chars.saturating_sub(3);
    if keep == 0 {
        String::from("...")
    } else {
        chars.truncate(keep);
        let mut s: String = chars.into_iter().collect();
        s.push_str("...");
        s
    }
}

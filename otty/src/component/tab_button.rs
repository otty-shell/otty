use iced::alignment;
use iced::widget::{Space, button, container, row, stack};
use iced::widget::text::Wrapping;
use iced::{Alignment, Length, advanced::graphics::core::Element, widget::{svg, text}};

use crate::{icons, theme::{AppTheme, IcedColorPalette}};

const TAB_LABEL_FONT_SIZE: f32 = 13.0;
const TAB_HEIGHT: f32 = 25.0;
const TAB_WIDTH: f32 = 235.0;
const CLOSE_ICON_SIZE: f32 = 25.0;
const CLOSE_BUTTON_RIGHT_PADDING: f32 = 2.0;
const DEFAULT_MAX_CHAR_COUNT_BEFORE_ELIPSIZE: usize = 20;

#[derive(Debug, Clone)]
pub enum TabButtonEvent {
    ActivateTab(u64),
    CloseTab(u64),
}

#[derive(Clone, Copy)]
struct TabButtonState<'a> {
    id: u64,
    title: &'a str,
    theme: &'a AppTheme,
    is_active: bool,
}

impl<'a> TabButtonState<'a> {
    fn new(
        id: u64,
        title: &'a str,
        is_active: bool,
        theme: &'a AppTheme,
    ) -> Self {
        Self {
            id,
            title,
            theme,
            is_active
        }
    }
}

pub struct TabButton<'a> {
    state: TabButtonState<'a>,
}

impl<'a> TabButton<'a> {
    pub fn new(
        id: u64,
        title: &'a str,
        is_active: bool,
        theme: &'a AppTheme,
    ) -> Self {
        Self {
            state: TabButtonState::new(id, title, is_active, theme)
        }
    }

    pub fn view(&self) -> Element<'a, TabButtonEvent, iced::Theme, iced::Renderer> {
        let state = self.state;

        let label = text(ellipsize(state.title))
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
                let theme = state.theme.iced_palette().clone();
                move |_, status| {
                    let color = if status == svg::Status::Hovered {
                        theme.red
                    } else if state.is_active {
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
            .on_press(TabButtonEvent::CloseTab(state.id))
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
                let theme = state.theme.iced_palette().clone();
                move |_| {
                    if state.is_active {
                        active_tab_style(&theme)
                    } else {
                        inactive_tab_style(&theme)
                    }
                }
            });

        button(pill)
            .on_press(TabButtonEvent::ActivateTab(state.id))
            .padding(0)
            .width(TAB_WIDTH)
            .height(TAB_HEIGHT)
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

use iced::border::Radius;
use iced::widget::text::Wrapping;
use iced::widget::{
    Space, button, container, row, scrollable, stack, svg, text,
};
use iced::{Alignment, Element, Length, alignment};

use crate::icons;
use crate::theme::{StyleOverrides, ThemeProps};

pub(crate) const TAB_BAR_HEIGHT: f32 = 25.0;
pub(crate) const TAB_BAR_SCROLL_ID: &str = "tab_bar_scroll";

const TAB_BUTTON_HEIGHT: f32 = 25.0;
const TAB_BUTTON_WIDTH: f32 = 235.0;
const TAB_BUTTON_PADDING: f32 = 0.0;
const TAB_LABEL_FONT_SIZE: f32 = 13.0;
const TAB_PILL_PADDING: f32 = 2.0;
const TAB_CLOSE_ICON_SIZE: f32 = 25.0;
const TAB_CLOSE_BUTTON_RIGHT_PADDING: f32 = 2.0;
const TAB_CLOSE_BUTTON_PADDING: f32 = 0.0;

/// Props for rendering the tab bar.
#[derive(Debug, Clone)]
pub(crate) struct TabBarProps<'a> {
    pub(crate) tabs: Vec<(u64, &'a str)>,
    pub(crate) active_tab_id: u64,
    pub(crate) theme: ThemeProps<'a>,
}

/// Events emitted by the tab bar widget.
#[derive(Debug, Clone)]
pub(crate) enum TabBarEvent {
    ActivateTab { tab_id: u64 },
    CloseTab { tab_id: u64 },
}

pub(crate) fn view<'a>(props: TabBarProps<'a>) -> Element<'a, TabBarEvent> {
    let mut tabs_row = row![].spacing(0);

    for tab in &props.tabs {
        let (id, title) = tab;
        tabs_row = tabs_row.push(tab_button(
            *id,
            title,
            props.active_tab_id == *id,
            props.theme,
        ));
    }

    let scroll = scrollable::Scrollable::with_direction(
        tabs_row,
        scrollable::Direction::Horizontal(
            scrollable::Scrollbar::new()
                .width(0)
                .scroller_width(0)
                .margin(0),
        ),
    )
    .id(TAB_BAR_SCROLL_ID)
    .width(Length::Fill);

    let palette = props.theme.theme.iced_palette();

    container(scroll)
        .height(Length::Fixed(TAB_BAR_HEIGHT))
        .width(Length::Fill)
        .style(move |_| iced::widget::container::Style {
            background: Some(palette.dim_black.into()),
            text_color: None,
            ..Default::default()
        })
        .into()
}

/// A clickable tab pill with close affordance.
fn tab_button<'a>(
    tab_id: u64,
    title: &str,
    is_active: bool,
    theme_props: ThemeProps<'a>,
) -> Element<'a, TabBarEvent> {
    let palette = theme_props.theme.iced_palette();
    let foreground = palette.foreground;
    let dim_foreground = palette.dim_foreground;
    let red = palette.red;
    let background = palette.background;
    let dim_black = palette.dim_black;

    let label = text(ellipsize(title))
        .size(TAB_LABEL_FONT_SIZE)
        .width(Length::Fill)
        .height(Length::Shrink)
        .align_y(Alignment::Center)
        .align_x(Alignment::Center)
        .wrapping(Wrapping::None);

    let close_icon = svg::Handle::from_memory(icons::WINDOW_CLOSE);
    let close_svg = svg::Svg::new(close_icon)
        .width(Length::Fixed(TAB_CLOSE_ICON_SIZE))
        .height(Length::Fixed(TAB_CLOSE_ICON_SIZE))
        .style({
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
        .on_press(TabBarEvent::CloseTab { tab_id })
        .padding(TAB_CLOSE_BUTTON_PADDING)
        .height(Length::Fill)
        .style(|_, _| iced::widget::button::Style::default());

    let close_button_row = row![
        Space::new().width(Length::Fill),
        close_button,
        Space::new().width(Length::Fixed(TAB_CLOSE_BUTTON_RIGHT_PADDING))
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
        .padding(TAB_PILL_PADDING)
        .width(Length::Fill)
        .height(Length::Fill)
        .style({
            let overrides = theme_props.overrides;
            move |_| {
                if is_active {
                    tab_button_style(background, foreground, overrides)
                } else {
                    tab_button_style(dim_black, dim_foreground, overrides)
                }
            }
        });

    button(pill)
        .on_press(TabBarEvent::ActivateTab { tab_id })
        .padding(TAB_BUTTON_PADDING)
        .width(TAB_BUTTON_WIDTH)
        .height(TAB_BUTTON_HEIGHT)
        .into()
}

fn tab_button_style(
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

const DEFAULT_MAX_CHAR_COUNT_BEFORE_ELLIPSIZE: usize = 20;

fn ellipsize(s: &str) -> String {
    let total = s.chars().count();
    if total <= DEFAULT_MAX_CHAR_COUNT_BEFORE_ELLIPSIZE {
        return s.to_owned();
    }

    let keep = DEFAULT_MAX_CHAR_COUNT_BEFORE_ELLIPSIZE - 2;
    let tail: String = s
        .chars()
        .rev()
        .take(keep)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    format!("..{tail}")
}

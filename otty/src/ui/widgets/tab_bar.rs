use iced::border::Radius;
use iced::widget::text::Wrapping;
use iced::widget::{
    Space, button, container, row, scrollable, stack, svg, text,
};
use iced::{Alignment, Element, Length, alignment};

use crate::features::tab::TabEvent;
use crate::icons;
use crate::theme::{StyleOverrides, ThemeProps};

/// Layout metrics for the tab bar.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TabBarMetrics {
    pub(crate) height: f32,
}

impl Default for TabBarMetrics {
    fn default() -> Self {
        Self { height: 25.0 }
    }
}

pub(crate) fn tab_bar_metrics() -> TabBarMetrics {
    TabBarMetrics::default()
}

/// Props for rendering the tab bar.
#[derive(Debug, Clone)]
pub(crate) struct Props<'a> {
    pub(crate) tabs: Vec<(u64, String)>,
    pub(crate) active_tab_id: u64,
    pub(crate) theme: ThemeProps<'a>,
}

pub(crate) fn view<'a>(props: Props<'a>) -> Element<'a, TabEvent> {
    let metrics = tab_bar_metrics();
    let mut tabs_row = row![].spacing(0);

    for tab in &props.tabs {
        let (id, title) = tab;
        tabs_row = tabs_row.push(tab_button(
            *id,
            title.clone(),
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
    .width(Length::Fill);

    let palette = props.theme.theme.iced_palette();

    container(scroll)
        .height(Length::Fixed(metrics.height))
        .width(Length::Fill)
        .style(move |_| iced::widget::container::Style {
            background: Some(palette.dim_black.into()),
            text_color: None,
            ..Default::default()
        })
        .into()
}

/// Layout metrics for a tab button.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TabButtonMetrics {
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

/// A clickable tab pill with close affordance.
fn tab_button<'a>(
    tab_id: u64,
    title: String,
    is_active: bool,
    theme_props: ThemeProps<'a>,
) -> Element<'a, TabEvent> {
    let metrics = TabButtonMetrics::default();
    let palette = theme_props.theme.iced_palette();
    let foreground = palette.foreground;
    let dim_foreground = palette.dim_foreground;
    let red = palette.red;
    let background = palette.background;
    let dim_black = palette.dim_black;

    let label = text(ellipsize(&title))
        .size(metrics.label_font_size)
        .width(Length::Fill)
        .height(Length::Shrink)
        .align_y(Alignment::Center)
        .align_x(Alignment::Center)
        .wrapping(Wrapping::None);

    let close_icon = svg::Handle::from_memory(icons::WINDOW_CLOSE);
    let close_svg = svg::Svg::new(close_icon)
        .width(Length::Fixed(metrics.close_icon_size))
        .height(Length::Fixed(metrics.close_icon_size))
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
        .on_press(TabEvent::CloseTab { tab_id })
        .padding(metrics.close_button_padding)
        .height(Length::Fill)
        .style(|_, _| iced::widget::button::Style::default());

    let close_button_row = row![
        Space::new().width(Length::Fill),
        close_button,
        Space::new().width(Length::Fixed(metrics.close_button_right_padding))
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
        .padding(metrics.pill_padding)
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
        .on_press(TabEvent::ActivateTab { tab_id })
        .padding(metrics.padding)
        .width(metrics.width)
        .height(metrics.height)
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

const DEFAULT_MAX_CHAR_COUNT_BEFORE_ELIPSIZE: usize = 20;

pub fn ellipsize(s: &str) -> String {
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

    format!("..{tail}")
}

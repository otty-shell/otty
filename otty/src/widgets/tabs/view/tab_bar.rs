use iced::widget::text::Wrapping;
use iced::widget::{
    Space, button, container, mouse_area, row, scrollable, stack, svg, text,
};
use iced::{Alignment, Element, Length, alignment};

use super::super::event::TabsIntent;
use super::super::model::TabBarItem;
use crate::icons;
use crate::layout::{BUTTON_SIZE_COMPACT, RADIUS_CONTROL, TAB_BAR_HEIGHT};
use crate::theme::{ThemeProps, mix_color};

pub(crate) const TAB_BAR_SCROLL_ID: &str = "tab_bar_scroll";

const TAB_BUTTON_HEIGHT: f32 = BUTTON_SIZE_COMPACT;
const TAB_BUTTON_WIDTH: f32 = 235.0;
const TAB_BUTTON_PADDING: f32 = 0.0;
const TAB_LABEL_FONT_SIZE: f32 = 13.0;
const TAB_PILL_PADDING: f32 = 2.0;
const TAB_CLOSE_ICON_SIZE: f32 = 18.0;
const TAB_CLOSE_BUTTON_RIGHT_PADDING: f32 = 2.0;
const TAB_CLOSE_BUTTON_PADDING: f32 = 0.0;

/// Props for rendering the tab bar.
#[derive(Debug, Clone)]
pub(crate) struct TabBarProps<'a> {
    pub(crate) tabs: Vec<TabBarItem>,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the tab bar as a horizontal scrollable row.
pub(crate) fn view<'a>(props: TabBarProps<'a>) -> Element<'a, TabsIntent> {
    let mut tabs_row = row![].spacing(0);

    for tab in &props.tabs {
        tabs_row = tabs_row.push(tab_button(
            tab.id,
            tab.title.clone(),
            tab.is_active,
            tab.is_hovered,
            tab.close_visible,
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

    container(scroll)
        .height(Length::Fixed(TAB_BAR_HEIGHT))
        .width(Length::Fill)
        .style(move |_| iced::widget::container::Style {
            // Modern UI：标签栏透明，仅药丸标签可见
            background: Some(iced::Color::TRANSPARENT.into()),
            text_color: None,
            ..Default::default()
        })
        .into()
}

/// A clickable tab pill with close affordance.
fn tab_button<'a>(
    tab_id: u64,
    title: String,
    is_active: bool,
    is_hovered: bool,
    close_visible: bool,
    theme_props: ThemeProps<'a>,
) -> Element<'a, TabsIntent> {
    let palette = theme_props.theme.ui_palette();
    let foreground = palette.foreground;
    let muted_foreground = palette.muted_foreground;
    let danger = palette.danger;
    let surface_background = palette.surface_background;

    let label = text(title)
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
                    danger
                } else if is_active {
                    foreground
                } else {
                    muted_foreground
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
        .on_press(TabsIntent::CloseTab { tab_id })
        .padding(TAB_CLOSE_BUTTON_PADDING)
        .height(Length::Fill)
        .style(|_, _| iced::widget::button::Style::default());

    let close_button_row = if close_visible {
        row![
            Space::new().width(Length::Fill),
            close_button,
            Space::new().width(Length::Fixed(TAB_CLOSE_BUTTON_RIGHT_PADDING))
        ]
    } else {
        row![
            Space::new().width(Length::Fill),
            Space::new().width(Length::Fixed(TAB_CLOSE_BUTTON_RIGHT_PADDING))
        ]
    }
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
        .style(move |_| {
            if is_active {
                // Modern UI 药丸：前景色 16% 混合背景（color-mix 等价）
                tab_button_style(
                    mix_color(foreground, surface_background, 0.16),
                    foreground,
                )
            } else if is_hovered {
                tab_button_style(
                    mix_color(foreground, surface_background, 0.08),
                    foreground,
                )
            } else {
                // 未激活：完全透明，仅悬停/激活才显示高亮
                tab_button_style(iced::Color::TRANSPARENT, muted_foreground)
            }
        });

    let button = button(pill)
        .on_press(TabsIntent::ActivateTab { tab_id })
        .clip(true)
        .padding(TAB_BUTTON_PADDING)
        .width(TAB_BUTTON_WIDTH)
        .height(TAB_BUTTON_HEIGHT);

    mouse_area(button)
        .on_enter(TabsIntent::TabHovered {
            tab_id: Some(tab_id),
        })
        .on_exit(TabsIntent::TabHovered { tab_id: None })
        .into()
}

fn tab_button_style(
    background: iced::Color,
    foreground: iced::Color,
) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(background.into()),
        text_color: Some(foreground),
        // Modern UI 标签：控件级圆角 4px
        border: iced::Border {
            radius: iced::border::Radius::from(RADIUS_CONTROL),
            ..Default::default()
        },
        ..Default::default()
    }
}

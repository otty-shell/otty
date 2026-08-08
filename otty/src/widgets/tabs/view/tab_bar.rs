use iced::widget::text::Wrapping;
use iced::widget::{
    Space, button, container, mouse_area, row, scrollable, stack, svg, text,
};
use iced::{Alignment, Color, Element, Length, alignment};

use super::super::event::TabsIntent;
use super::super::model::TabBarItem;
use crate::icons;
use crate::layout::{
    RADIUS_CONTROL, TAB_ACTION_RADIUS, TAB_ACTION_WIDTH, TAB_BAR_HEIGHT,
    TAB_BAR_HORIZONTAL_PADDING, TAB_BAR_VERTICAL_PADDING, TAB_ITEM_HEIGHT,
    TAB_ITEM_SPACING,
};
use crate::theme::{ThemeProps, desaturated, mix_color, readable_text_on};

pub(crate) const TAB_BAR_SCROLL_ID: &str = "tab_bar_scroll";

const TAB_BUTTON_WIDTH: f32 = 235.0;
const TAB_LABEL_FONT_SIZE: f32 = 13.0;
const TAB_CLOSE_ICON_SIZE: f32 = 16.0;
const TAB_CLOSE_BUTTON_RIGHT_PADDING: f32 = 2.0;
const TAB_INACTIVE_LABEL_PERCENT: f32 = 0.50;
const TAB_ACTIVE_PILL_PERCENT: f32 = 0.55;
const TAB_HOVER_PILL_PERCENT: f32 = 0.30;

/// Props for rendering the tab bar.
#[derive(Debug, Clone)]
pub(crate) struct TabBarProps<'a> {
    pub(crate) tabs: Vec<TabBarItem>,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the tab bar as a horizontal scrollable row.
pub(crate) fn view<'a>(props: TabBarProps<'a>) -> Element<'a, TabsIntent> {
    let mut tabs_row = row![].spacing(TAB_ITEM_SPACING);

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
        .padding(iced::Padding {
            top: TAB_BAR_VERTICAL_PADDING,
            bottom: TAB_BAR_VERTICAL_PADDING,
            left: TAB_BAR_HORIZONTAL_PADDING,
            right: 0.0,
        })
        .style(move |_| iced::widget::container::Style {
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
    let danger = palette.danger;
    let surface_background = palette.surface_background;

    // 激活/悬停 pill 用 accent(玫红)混合而非前景灰,保证视觉上"玫红激活、灰未激活"
    let active_pill =
        mix_color(palette.accent, surface_background, TAB_ACTIVE_PILL_PERCENT);
    let hover_pill =
        mix_color(palette.accent, surface_background, TAB_HOVER_PILL_PERCENT);
    let active_label = readable_text_on(active_pill, palette);
    let hover_label = readable_text_on(hover_pill, palette);
    // 未激活标签:去饱和成中性灰,不继承玫红底色
    let inactive_label = desaturated(mix_color(
        palette.foreground,
        surface_background,
        TAB_INACTIVE_LABEL_PERCENT,
    ));

    let label = text(title)
        .size(TAB_LABEL_FONT_SIZE)
        .width(Length::Fill)
        .height(Length::Shrink)
        .align_y(Alignment::Center)
        .align_x(Alignment::Center)
        .wrapping(Wrapping::None);

    let close_icon = svg::Handle::from_memory(icons::WINDOW_CLOSE);

    // Close affordance is an overlay surface on the pill's right edge that
    // follows the tab background so the glyph stays readable on the surface.
    let close_surface = container(
        container(
            svg::Svg::new(close_icon)
                .width(Length::Fixed(TAB_CLOSE_ICON_SIZE))
                .height(Length::Fixed(TAB_CLOSE_ICON_SIZE))
                .style(move |_, status| {
                    let color = if status == svg::Status::Hovered {
                        danger
                    } else if is_active {
                        active_label
                    } else {
                        hover_label
                    };
                    svg::Style { color: Some(color) }
                }),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center),
    )
    .width(Length::Fixed(TAB_ACTION_WIDTH))
    .height(Length::Fill)
    .style(move |_| {
        let background = if close_visible {
            if is_active { active_pill } else { hover_pill }
        } else {
            Color::TRANSPARENT
        };

        iced::widget::container::Style {
            background: Some(background.into()),
            border: iced::Border {
                radius: iced::border::Radius {
                    top_right: TAB_ACTION_RADIUS,
                    bottom_right: TAB_ACTION_RADIUS,
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        }
    });

    let close_button = button(close_surface)
        .on_press(TabsIntent::CloseTab { tab_id })
        .padding(0)
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
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_| {
            if is_active {
                tab_button_style(active_pill, active_label)
            } else if is_hovered {
                tab_button_style(hover_pill, hover_label)
            } else {
                // 未激活：完全透明，灰色文字，悬停/激活才显示玫红 pill
                tab_button_style(Color::TRANSPARENT, inactive_label)
            }
        });

    let button = button(pill)
        .on_press(TabsIntent::ActivateTab { tab_id })
        .style(|_, _| iced::widget::button::Style {
            // 不用默认 primary(玫红)底: pill 才负责描背景
            ..Default::default()
        })
        .clip(true)
        .padding(0)
        .width(TAB_BUTTON_WIDTH)
        .height(TAB_ITEM_HEIGHT);

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

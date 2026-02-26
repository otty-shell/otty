use iced::widget::{MouseArea, Space, Stack, container, row, svg, text};
use iced::{Element, Length, alignment, mouse};

use super::super::event::ChromeEvent;
use crate::components::primitive::icon_button::{
    IconButtonEvent, IconButtonProps, IconButtonVariant,
    view as icon_button_view,
};
use crate::shared::ui::fonts::FontsConfig;
use crate::shared::ui::icons::{
    LOGO_SMALL, WINDOW_CLOSE, WINDOW_FULLSCREEN, WINDOW_TRAY,
};
use crate::shared::ui::theme::{StyleOverrides, ThemeProps};

pub(crate) const ACTION_BAR_HEIGHT: f32 = 30.0;
const ACTION_BAR_TITLE_SCALE: f32 = 0.9;
const ACTION_BAR_CONTROL_BUTTON_SIZE: f32 = 24.0;
const ACTION_BAR_CONTROL_ICON_SIZE: f32 = 18.0;
const ACTION_BAR_LOGO_ICON_SIZE: f32 = 18.0;
const ACTION_BAR_HORIZONTAL_PADDING: f32 = 12.0;
const ACTION_BAR_RIGHT_PADDING: f32 = 8.0;
const ACTION_BAR_LEFT_SPACING: f32 = 8.0;
const ACTION_BAR_CONTROLS_SPACING: f32 = 6.0;

/// Props for rendering the action bar.
#[derive(Debug, Clone)]
pub(crate) struct ActionBarProps<'a> {
    pub(crate) title: String,
    pub(crate) theme: ThemeProps<'a>,
    pub(crate) fonts: &'a FontsConfig,
}

/// Render the draggable window header with controls.
pub(crate) fn view<'a>(props: ActionBarProps<'a>) -> Element<'a, ChromeEvent> {
    let title_font_size = props.fonts.ui.size * ACTION_BAR_TITLE_SCALE;
    let palette = props.theme.theme.iced_palette();
    let overrides = props.theme.overrides;
    let dim_foreground = palette.dim_foreground;
    let dim_black = palette.dim_black;

    let detail_label = text(props.title)
        .size(title_font_size)
        .font(props.fonts.ui.font_type);

    let logo = svg::Svg::new(svg::Handle::from_memory(LOGO_SMALL))
        .width(Length::Fixed(ACTION_BAR_LOGO_ICON_SIZE))
        .height(Length::Fixed(ACTION_BAR_LOGO_ICON_SIZE));

    let logo_container = container(logo)
        .width(Length::Shrink)
        .height(Length::Fill)
        .align_x(alignment::Horizontal::Left)
        .align_y(alignment::Vertical::Center)
        .padding([0.0, ACTION_BAR_HORIZONTAL_PADDING]);
    let logo_container = MouseArea::new(logo_container)
        .on_press(ChromeEvent::ToggleSidebarVisibility)
        .interaction(mouse::Interaction::Pointer);

    let center_zone = container(detail_label)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .padding([0.0, ACTION_BAR_HORIZONTAL_PADDING])
        .style(move |_| iced::widget::container::Style {
            text_color: Some(resolve_text_color(dim_foreground, overrides)),
            ..Default::default()
        });

    let left_controls = row![logo_container]
        .spacing(ACTION_BAR_LEFT_SPACING)
        .align_y(alignment::Vertical::Center);

    let controls_row = row![
        icon_button(
            WINDOW_FULLSCREEN,
            IconButtonVariant::Standard,
            props.theme,
        )
        .map(|_| ChromeEvent::ToggleFullScreen),
        icon_button(WINDOW_TRAY, IconButtonVariant::Standard, props.theme)
            .map(|_| ChromeEvent::MinimizeWindow),
        icon_button(WINDOW_CLOSE, IconButtonVariant::Danger, props.theme)
            .map(|_| ChromeEvent::CloseWindow),
    ]
    .spacing(ACTION_BAR_CONTROLS_SPACING)
    .align_y(alignment::Vertical::Center);

    let controls_container = container(controls_row)
        .width(Length::Shrink)
        .height(Length::Fill)
        .align_x(alignment::Horizontal::Right)
        .align_y(alignment::Vertical::Center)
        .padding([0.0, ACTION_BAR_RIGHT_PADDING]);

    let drag_surface = MouseArea::new(
        container(Space::new().width(Length::Fill).height(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .on_press(ChromeEvent::StartWindowDrag)
    .on_double_click(ChromeEvent::ToggleFullScreen);

    let base_row = row![
        left_controls,
        Space::new().width(Length::Fill),
        controls_container
    ]
    .spacing(ACTION_BAR_LEFT_SPACING)
    .align_y(alignment::Vertical::Center)
    .width(Length::Fill)
    .height(Length::Fill);

    let content = Stack::new()
        .push(drag_surface)
        .push(base_row)
        .push(center_zone);

    container(content)
        .width(Length::Fill)
        .height(Length::Fixed(ACTION_BAR_HEIGHT))
        .style(move |_| iced::widget::container::Style {
            background: Some(resolve_background(dim_black, overrides).into()),
            ..Default::default()
        })
        .into()
}

fn icon_button<'a>(
    icon: &'static [u8],
    variant: IconButtonVariant,
    theme: ThemeProps<'a>,
) -> Element<'a, IconButtonEvent> {
    let props = IconButtonProps {
        icon,
        theme,
        size: ACTION_BAR_CONTROL_BUTTON_SIZE,
        icon_size: ACTION_BAR_CONTROL_ICON_SIZE,
        variant,
    };
    icon_button_view(props)
}

fn resolve_background(
    default_color: iced::Color,
    overrides: Option<StyleOverrides>,
) -> iced::Color {
    overrides
        .and_then(|o| o.background)
        .unwrap_or(default_color)
}

fn resolve_text_color(
    default_color: iced::Color,
    overrides: Option<StyleOverrides>,
) -> iced::Color {
    overrides
        .and_then(|o| o.foreground)
        .unwrap_or(default_color)
}

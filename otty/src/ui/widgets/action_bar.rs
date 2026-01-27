use iced::alignment;
use iced::widget::{MouseArea, Space, Stack, container, row, svg, text};
use iced::{Element, Length};

use crate::fonts::FontsConfig;
use crate::icons::{
    ADD_TAB_HEADER, LOGO_SMALL, WINDOW_CLOSE, WINDOW_FULLSCREEN, WINDOW_TRAY,
};
use crate::theme::{StyleOverrides, ThemeProps};
use crate::ui::components::icon_button::{
    IconButton, IconButtonEvent, IconButtonProps, IconButtonVariant,
};

pub(crate) const ACTION_BAR_HEIGHT: f32 = 30.0;
const ACTION_BAR_TITLE_SCALE: f32 = 0.9;
const ACTION_BAR_CONTROL_BUTTON_SIZE: f32 = 24.0;
const ACTION_BAR_CONTROL_ICON_SIZE: f32 = 18.0;
const ACTION_BAR_LOGO_ICON_SIZE: f32 = 18.0;
const ACTION_BAR_HORIZONTAL_PADDING: f32 = 12.0;
const ACTION_BAR_RIGHT_PADDING: f32 = 8.0;
const ACTION_BAR_LEFT_SPACING: f32 = 8.0;
const ACTION_BAR_CONTROLS_SPACING: f32 = 6.0;

/// UI events emitted by the window action bar.
#[derive(Debug, Clone)]
pub(crate) enum Event {
    NewTab,
    ToggleFullScreen,
    ToggleTray,
    CloseWindow,
    StartWindowDrag,
}

/// Props for rendering the action bar.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Props<'a> {
    pub(crate) title: &'a str,
    pub(crate) theme: ThemeProps<'a>,
    pub(crate) fonts: &'a FontsConfig,
}

/// The draggable window header with controls.
pub fn view<'a>(props: Props<'a>) -> Element<'a, Event> {
    let title_font_size = props.fonts.ui.size * ACTION_BAR_TITLE_SCALE;

    let palette = props.theme.theme.iced_palette();
    let overrides = props.theme.overrides;

    let detail_label = text(props.title).size(title_font_size);

    let logo = svg::Svg::new(svg::Handle::from_memory(LOGO_SMALL))
        .width(Length::Fixed(ACTION_BAR_LOGO_ICON_SIZE))
        .height(Length::Fixed(ACTION_BAR_LOGO_ICON_SIZE));

    let logo_container = container(logo)
        .width(Length::Shrink)
        .height(Length::Fill)
        .align_x(alignment::Horizontal::Left)
        .align_y(alignment::Vertical::Center)
        .padding([0.0, ACTION_BAR_HORIZONTAL_PADDING]);

    let center_zone = container(detail_label)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .padding([0.0, ACTION_BAR_HORIZONTAL_PADDING])
        .style(move |_| iced::widget::container::Style {
            text_color: Some(resolve_text_color(
                palette.dim_foreground,
                overrides,
            )),
            ..Default::default()
        });

    let add_tab_button =
        icon_button(ADD_TAB_HEADER, IconButtonVariant::Standard, props.theme)
            .map(|_| Event::NewTab);

    let left_controls = row![logo_container, add_tab_button]
        .spacing(ACTION_BAR_LEFT_SPACING)
        .align_y(alignment::Vertical::Center);

    let controls_row = row![
        icon_button(
            WINDOW_FULLSCREEN,
            IconButtonVariant::Standard,
            props.theme
        )
        .map(|_| Event::ToggleFullScreen),
        icon_button(WINDOW_TRAY, IconButtonVariant::Standard, props.theme)
            .map(|_| Event::ToggleTray),
        icon_button(WINDOW_CLOSE, IconButtonVariant::Danger, props.theme)
            .map(|_| Event::CloseWindow),
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
    .on_press(Event::StartWindowDrag)
    .on_double_click(Event::ToggleFullScreen);

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
            background: Some(
                resolve_background(palette.dim_black, overrides).into(),
            ),
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

    IconButton::new(props).view()
}

fn resolve_background(
    default_color: iced::Color,
    overrides: Option<StyleOverrides>,
) -> iced::Color {
    overrides
        .and_then(|overrides| overrides.background)
        .unwrap_or(default_color)
}

fn resolve_text_color(
    default_color: iced::Color,
    overrides: Option<StyleOverrides>,
) -> iced::Color {
    overrides
        .and_then(|overrides| overrides.foreground)
        .unwrap_or(default_color)
}

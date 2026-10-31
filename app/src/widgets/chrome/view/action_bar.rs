use iced::widget::{MouseArea, Space, Stack, container, row, svg, text};
use iced::{Element, Length, alignment, mouse};
use otty_ui_kit::components::icon_button::IconButton;
use otty_ui_kit::{fonts, size, spacing, typography};

use super::super::event::ChromeIntent;
use crate::i18n::{self, Key};
use crate::icons::{LOGO_SMALL, WINDOW_CLOSE, WINDOW_FULLSCREEN, WINDOW_TRAY};
use crate::theme::ThemeProps;

/// Logo height in the title bar, set by the design system's iconography
/// rules (logo-small, 20px high). No size token.
const LOGO_HEIGHT: f32 = 20.0;
/// Logo width; it follows the logo proportions (145.36 x 103.12).
const LOGO_WIDTH: f32 = LOGO_HEIGHT * 145.36 / 103.12;

/// Props for rendering the action bar.
#[derive(Debug, Clone)]
pub(crate) struct ActionBarProps<'a> {
    pub(crate) title: &'a str,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the draggable window header with controls.
pub(crate) fn view<'a>(props: ActionBarProps<'a>) -> Element<'a, ChromeIntent> {
    let palette = props.theme.theme.ui();
    let text_muted = palette.text_muted();
    let surface_chrome = palette.surface_chrome();

    let detail_label = text(props.title)
        .size(typography::BODY.size())
        .font(fonts::UI);

    let logo = svg::Svg::new(svg::Handle::from_memory(LOGO_SMALL))
        .width(Length::Fixed(LOGO_WIDTH))
        .height(Length::Fixed(LOGO_HEIGHT));

    let logo_container = container(logo)
        .width(Length::Shrink)
        .height(Length::Fill)
        .align_x(alignment::Horizontal::Left)
        .align_y(alignment::Vertical::Center)
        .padding([0.0, spacing::SPACE_4]);
    let logo_container = MouseArea::new(logo_container)
        .on_press(ChromeIntent::ToggleSidebarVisibility)
        .interaction(mouse::Interaction::Pointer);

    let center_zone = container(detail_label)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .padding([0.0, spacing::SPACE_4])
        .style(move |_| iced::widget::container::Style {
            text_color: Some(text_muted),
            ..Default::default()
        });

    let controls_row = row![
        IconButton::new(
            svg::Handle::from_memory(WINDOW_FULLSCREEN),
            i18n::t(Key::WindowToggleFullscreen),
            palette,
        )
        .on_press(ChromeIntent::ToggleFullScreen),
        IconButton::new(
            svg::Handle::from_memory(WINDOW_TRAY),
            i18n::t(Key::WindowMinimize),
            palette,
        )
        .on_press(ChromeIntent::MinimizeWindow),
        IconButton::new(
            svg::Handle::from_memory(WINDOW_CLOSE),
            i18n::t(Key::ClosePane),
            palette,
        )
        .on_press(ChromeIntent::CloseWindow),
    ]
    .spacing(spacing::SPACE_1)
    .align_y(alignment::Vertical::Center);

    let controls_container = container(controls_row)
        .width(Length::Shrink)
        .height(Length::Fill)
        .align_x(alignment::Horizontal::Right)
        .align_y(alignment::Vertical::Center)
        .padding([0.0, spacing::SPACE_2]);

    let drag_surface = MouseArea::new(
        container(Space::new().width(Length::Fill).height(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .on_press(ChromeIntent::StartWindowDrag)
    .on_double_click(ChromeIntent::ToggleFullScreen);

    let base_row = row![
        logo_container,
        Space::new().width(Length::Fill),
        controls_container
    ]
    .align_y(alignment::Vertical::Center)
    .width(Length::Fill)
    .height(Length::Fill);

    let content = Stack::new()
        .push(drag_surface)
        .push(base_row)
        .push(center_zone);

    container(content)
        .width(Length::Fill)
        .height(Length::Fixed(size::TITLEBAR))
        .style(move |_| iced::widget::container::Style {
            background: Some(surface_chrome.into()),
            ..Default::default()
        })
        .into()
}

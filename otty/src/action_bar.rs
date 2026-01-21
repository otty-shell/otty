use iced::alignment;
use iced::widget::{
    MouseArea, Space, Stack, button, container, row, svg, text,
};
use iced::{Element, Length, Theme};

use crate::icons::{
    ADD_TAB_HEADER, LOGO_SMALL, WINDOW_CLOSE, WINDOW_FULLSCREEN, WINDOW_TRAY,
};
use crate::app::{ActiveView, App, Event};
use crate::theme::IcedColorPalette;

pub const ACTION_BAR_HEIGHT: f32 = 30.0;
const CONTROL_BUTTON_SIZE: f32 = 24.0;
const CONTROL_ICON_SIZE: f32 = 18.0;
const LOGO_ICON_SIZE: f32 = 18.0;

pub fn view_action_bar(app: &App) -> Element<'_, Event, Theme, iced::Renderer> {
    let detail_text = match app.active_view {
        ActiveView::Terminal => app
            .tabs
            .get(app.active_tab_index)
            .map(|tab| tab.title.clone())
            .unwrap_or_else(|| app.title()),
    };
    let detail_label = text(detail_text).size(app.fonts.ui.size * 0.9);

    let logo = svg::Svg::new(svg::Handle::from_memory(LOGO_SMALL))
        .width(Length::Fixed(LOGO_ICON_SIZE))
        .height(Length::Fixed(LOGO_ICON_SIZE));

    let logo_container = container(logo)
        .width(Length::Shrink)
        .height(Length::Fill)
        .align_x(alignment::Horizontal::Left)
        .align_y(alignment::Vertical::Center)
        .padding([0, 12]);

    let theme = app.theme_manager.current().iced_palette();
    let center_zone = container(detail_label)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .padding([0, 12])
        .style(move |_| iced::widget::container::Style {
            text_color: Some(theme.dim_foreground),
            ..Default::default()
        });

    let add_tab_button = window_control_button(
        ADD_TAB_HEADER,
        Event::NewTab,
        theme,
        ControlVariant::Standard,
    );
    let left_controls = row![logo_container, add_tab_button]
        .spacing(8)
        .align_y(alignment::Vertical::Center);

    let controls_row = row![
        window_control_button(
            WINDOW_FULLSCREEN,
            Event::ToggleFullScreen,
            theme,
            ControlVariant::Standard
        ),
        window_control_button(
            WINDOW_TRAY,
            Event::ToggleTray,
            theme,
            ControlVariant::Standard
        ),
        window_control_button(
            WINDOW_CLOSE,
            Event::CloseWindow,
            theme,
            ControlVariant::Close
        ),
    ]
    .spacing(6)
    .align_y(alignment::Vertical::Center);

    let controls_container = container(controls_row)
        .width(Length::Shrink)
        .height(Length::Fill)
        .align_x(alignment::Horizontal::Right)
        .align_y(alignment::Vertical::Center)
        .padding([0, 8]);

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
    .spacing(8)
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
            background: Some(theme.dim_black.into()),
            ..Default::default()
        })
        .into()
}

#[derive(Debug, Clone, Copy)]
enum ControlVariant {
    Standard,
    Close,
}

fn window_control_button(
    icon_bytes: &'static [u8],
    message: Event,
    theme: &IcedColorPalette,
    variant: ControlVariant,
) -> Element<'static, Event, Theme, iced::Renderer> {
    let (base_color, hovered_color) = match variant {
        ControlVariant::Standard => (theme.dim_foreground, theme.blue),
        ControlVariant::Close => (theme.dim_foreground, theme.red),
    };

    let icon = svg::Svg::new(svg::Handle::from_memory(icon_bytes))
        .width(Length::Fixed(CONTROL_ICON_SIZE))
        .height(Length::Fixed(CONTROL_ICON_SIZE))
        .style(move |_, status| {
            let color = if matches!(status, svg::Status::Hovered) {
                hovered_color
            } else {
                base_color
            };

            svg::Style { color: Some(color) }
        });

    let icon_container = container(icon)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center);

    button(icon_container)
        .on_press(message)
        .padding(0)
        .width(Length::Fixed(CONTROL_BUTTON_SIZE))
        .height(Length::Fixed(CONTROL_BUTTON_SIZE))
        .style(|_, _| iced::widget::button::Style::default())
        .into()
}

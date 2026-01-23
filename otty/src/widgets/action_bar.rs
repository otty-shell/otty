use iced::alignment;
use iced::widget::{MouseArea, Space, Stack, container, row, svg, text};
use iced::{Element, Length};

use crate::app::theme::{StyleOverrides, ThemeProps};
use crate::components::icon_button::{
    IconButton, IconButtonEvent, IconButtonProps, IconButtonVariant,
};
use crate::icons::{
    ADD_TAB_HEADER, LOGO_SMALL, WINDOW_CLOSE, WINDOW_FULLSCREEN, WINDOW_TRAY,
};

/// UI events emitted by the window action bar.
#[derive(Debug, Clone)]
pub(crate) enum ActionBarEvent {
    NewTab,
    ToggleFullScreen,
    ToggleTray,
    CloseWindow,
    StartWindowDrag,
}

/// Layout metrics for the action bar.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ActionBarMetrics {
    pub(crate) height: f32,
    control_button_size: f32,
    control_icon_size: f32,
    logo_icon_size: f32,
    pub(crate) title_font_size: f32,
}

impl Default for ActionBarMetrics {
    fn default() -> Self {
        Self {
            height: 30.0,
            control_button_size: 24.0,
            control_icon_size: 18.0,
            logo_icon_size: 18.0,
            title_font_size: 13.0,
        }
    }
}

/// Props for rendering the action bar.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ActionBarProps<'a> {
    pub(crate) title: &'a str,
    pub(crate) theme: ThemeProps<'a>,
    pub(crate) metrics: ActionBarMetrics,
}

/// The draggable window header with controls.
pub(crate) struct ActionBar<'a> {
    props: ActionBarProps<'a>,
}

impl<'a> ActionBar<'a> {
    pub fn new(props: ActionBarProps<'a>) -> Self {
        Self { props }
    }

    pub fn view(&self) -> Element<'a, ActionBarEvent> {
        let palette = self.props.theme.theme.iced_palette();
        let metrics = self.props.metrics;
        let overrides = self.props.theme.overrides;

        let detail_label =
            text(self.props.title).size(self.props.metrics.title_font_size);

        let logo = svg::Svg::new(svg::Handle::from_memory(LOGO_SMALL))
            .width(Length::Fixed(metrics.logo_icon_size))
            .height(Length::Fixed(metrics.logo_icon_size));

        let logo_container = container(logo)
            .width(Length::Shrink)
            .height(Length::Fill)
            .align_x(alignment::Horizontal::Left)
            .align_y(alignment::Vertical::Center)
            .padding([0, 12]);

        let center_zone = container(detail_label)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(alignment::Horizontal::Center)
            .align_y(alignment::Vertical::Center)
            .padding([0, 12])
            .style(move |_| iced::widget::container::Style {
                text_color: Some(resolve_text_color(
                    palette.dim_foreground,
                    overrides,
                )),
                ..Default::default()
            });

        let add_tab_button = icon_button(
            ADD_TAB_HEADER,
            IconButtonVariant::Standard,
            metrics,
            self.props.theme,
        )
        .map(|_| ActionBarEvent::NewTab);

        let left_controls = row![logo_container, add_tab_button]
            .spacing(8)
            .align_y(alignment::Vertical::Center);

        let controls_row = row![
            icon_button(
                WINDOW_FULLSCREEN,
                IconButtonVariant::Standard,
                metrics,
                self.props.theme,
            )
            .map(|_| ActionBarEvent::ToggleFullScreen),
            icon_button(
                WINDOW_TRAY,
                IconButtonVariant::Standard,
                metrics,
                self.props.theme,
            )
            .map(|_| ActionBarEvent::ToggleTray),
            icon_button(
                WINDOW_CLOSE,
                IconButtonVariant::Danger,
                metrics,
                self.props.theme,
            )
            .map(|_| ActionBarEvent::CloseWindow),
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
        .on_press(ActionBarEvent::StartWindowDrag)
        .on_double_click(ActionBarEvent::ToggleFullScreen);

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
            .height(Length::Fixed(metrics.height))
            .style(move |_| iced::widget::container::Style {
                background: Some(
                    resolve_background(palette.dim_black, overrides).into(),
                ),
                ..Default::default()
            })
            .into()
    }
}

fn icon_button<'a>(
    icon: &'static [u8],
    variant: IconButtonVariant,
    metrics: ActionBarMetrics,
    theme: ThemeProps<'a>,
) -> Element<'a, IconButtonEvent> {
    let props = IconButtonProps {
        icon,
        theme,
        size: metrics.control_button_size,
        icon_size: metrics.control_icon_size,
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

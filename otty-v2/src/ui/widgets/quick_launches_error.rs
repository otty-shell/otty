use iced::widget::{column, container, text};
use iced::{Element, Length, alignment};

use crate::theme::ThemeProps;
use crate::widgets::quick_launch::QuickLaunchErrorState;

/// Props for rendering a quick launch error tab.
#[derive(Debug, Clone, Copy)]
pub(crate) struct QuickLaunchesErrorProps<'a> {
    pub(crate) error: &'a QuickLaunchErrorState,
    pub(crate) theme: ThemeProps<'a>,
}

/// Events emitted by quick launch error widget.
#[derive(Debug, Clone)]
pub(crate) enum QuickLaunchesErrorEvent {}

pub(crate) fn view<'a>(
    props: QuickLaunchesErrorProps<'a>,
) -> Element<'a, QuickLaunchesErrorEvent> {
    let palette = props.theme.theme.iced_palette();
    let title = text(props.error.title()).size(18.0).style(move |_| {
        iced::widget::text::Style {
            color: Some(palette.red),
        }
    });

    let message = text(props.error.message()).size(13.0).style(move |_| {
        iced::widget::text::Style {
            color: Some(palette.bright_red),
        }
    });

    let content = column![title, message]
        .spacing(8)
        .width(Length::Fill)
        .align_x(alignment::Horizontal::Left);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(24)
        .into()
}

use iced::alignment;
use iced::widget::{column, container, text};
use iced::{Element, Length};

use crate::features::quick_launches::QuickLaunchErrorState;
use crate::theme::ThemeProps;

/// Props for rendering a quick launch error tab.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Props<'a> {
    pub(crate) error: &'a QuickLaunchErrorState,
    pub(crate) theme: ThemeProps<'a>,
}

pub(crate) fn view<'a>(props: Props<'a>) -> Element<'a, crate::app::Event> {
    let palette = props.theme.theme.iced_palette();
    let title = text(&props.error.title).size(18.0).style(move |_| {
        iced::widget::text::Style {
            color: Some(palette.red),
        }
    });

    let message = text(&props.error.message).size(13.0).style(move |_| {
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

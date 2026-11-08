use iced::widget::{column, container, text};
use iced::{Element, Length};
use otty_ui_kit::{fonts, spacing, typography};

use super::super::event::QuickLaunchIntent;
use super::super::model::ErrorTabViewModel;
use crate::theme::ThemeProps;

/// Props for the quick launch error tab view.
pub(crate) struct ErrorTabProps<'a> {
    pub(crate) vm: ErrorTabViewModel<'a>,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the quick launch error tab content: a title and the error detail
/// in the terminal font.
pub(crate) fn view(props: ErrorTabProps<'_>) -> Element<'_, QuickLaunchIntent> {
    let palette = props.theme.theme.ui();

    let title = text(props.vm.title)
        .size(typography::TITLE.size())
        .font(fonts::UI)
        .color(palette.text_strong());
    let detail = text(props.vm.message)
        .size(typography::TERM_SMALL.size())
        .font(fonts::MONO)
        .color(palette.text_body());

    let content = column![title, detail]
        .spacing(spacing::SPACE_3)
        .width(Length::Fill);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(spacing::SPACE_6)
        .into()
}

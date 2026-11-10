pub(crate) mod pane_context_menu;
pub(crate) mod pane_grid;

use iced::widget::{container, text};
use iced::{Element, Length, alignment};
use otty_app_i18n::{self as i18n, Key};

use crate::event::TerminalIntent;

/// The line shown in place of a terminal tab whose terminal is missing.
pub(crate) fn missing_tab<'a>() -> Element<'a, TerminalIntent> {
    container(text(i18n::t(Key::TerminalTabNotInitialized)))
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .into()
}

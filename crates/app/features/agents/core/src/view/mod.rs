pub(crate) mod left_panel;
mod limits;
pub(crate) mod main_area;
pub(crate) mod profile_details;
pub(crate) mod remove_dialog;
pub(crate) mod status_bar;

use iced::widget::{row, text};
use iced::{Color, Element, Length};
use otty_app_i18n::{self as i18n, Key};
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, spacing, typography};

use super::event::AgentsIntent;

/// Width of the labels before the values in the details of a profile.
const LABEL_WIDTH: f32 = 140.0;

/// One labelled value, such as the config folder.
fn fact<'a>(
    label: Key,
    value: String,
    color: Color,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let body = typography::BODY;

    row![
        text(i18n::t(label))
            .size(body.size())
            .line_height(iced::Pixels(body.line_height()))
            .font(fonts::UI)
            .color(palette.text_muted())
            .width(LABEL_WIDTH),
        text(value)
            .size(body.size())
            .line_height(iced::Pixels(body.line_height()))
            .font(fonts::UI)
            .color(color)
            .width(Length::Fill),
    ]
    .spacing(spacing::SPACE_3)
    .into()
}

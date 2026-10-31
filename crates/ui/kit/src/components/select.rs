//! `Select`: pick one option from a longer list.

use std::fmt::Display;

use iced::widget::overlay::menu;
use iced::widget::pick_list::{self, PickList};
use iced::{Border, Element, Padding, Pixels};

use crate::palette::UiPalette;
use crate::{fonts, radius, typography};

/// Horizontal padding (`padding: 0 10px 0 11px`).
const PADDING_X: f32 = 11.0;
/// Vertical padding that makes a 13/20 text line 34px high.
const PADDING_Y: f32 = 7.0;

/// Renders a dropdown over `options` with the Menu look.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use otty_ui_kit::components::select::select;
/// use otty_ui_kit::palette::UiPalette;
///
/// #[derive(Clone)]
/// enum Message {
///     Picked(&'static str),
/// }
///
/// let palette = UiPalette::otty_dark();
/// let options = ["This computer", "prod-1"];
/// let dropdown: Element<'_, Message> =
///     select(&options, Some("prod-1"), Message::Picked, &palette);
/// ```
pub fn select<'a, T, Message>(
    options: &'a [T],
    selected: Option<T>,
    on_select: impl Fn(T) -> Message + 'a,
    palette: &UiPalette,
) -> Element<'a, Message>
where
    T: Clone + Display + PartialEq + 'a,
    Message: Clone + 'a,
{
    let field_palette = palette.clone();
    let menu_palette = palette.clone();
    let body = typography::BODY;

    PickList::new(options, selected, on_select)
        .font(fonts::UI)
        .text_size(Pixels(body.size()))
        .text_line_height(Pixels(body.line_height()))
        .padding(Padding::from([PADDING_Y, PADDING_X]))
        .style(move |_, status| field_style(&field_palette, status))
        .menu_style(move |_| menu_style(&menu_palette))
        .into()
}

/// Resolves the closed-field style.
fn field_style(
    palette: &UiPalette,
    status: pick_list::Status,
) -> pick_list::Style {
    let hovered = !matches!(status, pick_list::Status::Active);

    pick_list::Style {
        text_color: palette.text_strong(),
        placeholder_color: palette.text_muted(),
        handle_color: palette.text_muted(),
        background: if hovered {
            palette.surface_hover()
        } else {
            palette.surface_raised()
        }
        .into(),
        border: Border {
            radius: radius::RADIUS_SM.into(),
            ..Border::default()
        },
    }
}

/// Resolves the dropdown style, the same look as a Menu.
fn menu_style(palette: &UiPalette) -> menu::Style {
    menu::Style {
        background: palette.surface_overlay().into(),
        border: Border {
            radius: radius::RADIUS_MD.into(),
            ..Border::default()
        },
        text_color: palette.text_body(),
        selected_text_color: palette.text_strong(),
        selected_background: palette.surface_hover().into(),
        shadow: palette.shadow_popover(),
    }
}

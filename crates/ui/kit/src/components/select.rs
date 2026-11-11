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
    styled(options, selected, on_select, palette)
}

/// Renders a dropdown like [`select`] over options the dropdown owns, for
/// lists the caller builds while drawing (for example an engine's
/// networks).
///
/// # Example
///
/// ```
/// use iced::Element;
/// use otty_ui_kit::components::select::select_owned;
/// use otty_ui_kit::palette::UiPalette;
///
/// #[derive(Clone)]
/// enum Message {
///     Picked(String),
/// }
///
/// let palette = UiPalette::otty_dark();
/// let networks = vec!["bridge".to_owned(), "otty-net".to_owned()];
/// let dropdown: Element<'_, Message> = select_owned(
///     networks,
///     Some("bridge".to_owned()),
///     Message::Picked,
///     &palette,
/// );
/// ```
pub fn select_owned<'a, T, Message>(
    options: Vec<T>,
    selected: Option<T>,
    on_select: impl Fn(T) -> Message + 'a,
    palette: &UiPalette,
) -> Element<'a, Message>
where
    T: Clone + Display + PartialEq + 'a,
    Message: Clone + 'a,
{
    styled(options, selected, on_select, palette)
}

/// Builds the pick list with the Menu look over borrowed or owned options.
fn styled<'a, T, L, Message>(
    options: L,
    selected: Option<T>,
    on_select: impl Fn(T) -> Message + 'a,
    palette: &UiPalette,
) -> Element<'a, Message>
where
    T: Clone + Display + PartialEq + 'a,
    L: std::borrow::Borrow<[T]> + 'a,
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

#[cfg(test)]
mod tests {
    use iced::Point;
    use iced_test::simulator;

    use super::*;

    /// Middle of the second menu row under the 34px field.
    const SECOND_ROW_Y: f32 = 34.0 + 34.0 + 17.0;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Message {
        Picked(String),
    }

    #[test]
    fn given_owned_options_when_one_is_picked_then_it_is_sent() {
        for palette in [UiPalette::otty_dark(), UiPalette::otty_light()] {
            let networks = vec!["bridge".to_owned(), "otty-fx-net".to_owned()];
            let dropdown = select_owned(
                networks,
                Some("bridge".to_owned()),
                Message::Picked,
                &palette,
            );
            let mut ui = simulator(dropdown);

            // The pick list draws its text itself, so it is pressed by
            // position: the 34px field, then the second 34px menu row
            // under it.
            ui.point_at(Point::new(20.0, 17.0));
            let _ = ui.simulate(iced_test::simulator::click());
            // The menu takes the row under the pointer when it moves.
            let row = Point::new(20.0, SECOND_ROW_Y);
            ui.point_at(row);
            let _ = ui.simulate([iced::Event::Mouse(
                iced::mouse::Event::CursorMoved { position: row },
            )]);
            let _ = ui.simulate(iced_test::simulator::click());

            let messages: Vec<Message> = ui.into_messages().collect();
            assert_eq!(
                messages,
                vec![Message::Picked("otty-fx-net".to_owned())]
            );
        }
    }
}

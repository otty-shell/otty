//! `Segmented`: pick one of two to four short options.

use std::fmt::Display;

use iced::widget::{button, container, row};
use iced::{Alignment, Border, Element, Length, Padding};

use crate::palette::UiPalette;
use crate::{fonts, radius, spacing};

/// Frame padding (`padding: 3px`).
const FRAME_PADDING: f32 = 3.0;
/// Gap between segments (`gap: 2px`).
const SEGMENT_GAP: f32 = 2.0;
/// Segment height (`height: 28px`).
const SEGMENT_HEIGHT: f32 = 28.0;
/// Segment horizontal padding (`padding: 0 12px`).
const SEGMENT_PADDING_X: f32 = spacing::SPACE_4;
/// Segment text size (`font-size: 12.5px`).
const SEGMENT_TEXT_SIZE: f32 = 12.5;

/// Renders `options` as one row of segments; the one equal to `selected` is
/// highlighted. Pressing a segment sends `on_select(option)`.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use otty_ui_kit::components::segmented::segmented;
/// use otty_ui_kit::palette::UiPalette;
///
/// #[derive(Clone)]
/// enum Message {
///     Picked(&'static str),
/// }
///
/// let palette = UiPalette::otty_dark();
/// let options = ["Command", "Script"];
/// let row: Element<'_, Message> =
///     segmented(&options, &"Command", Message::Picked, &palette);
/// ```
pub fn segmented<'a, T, Message>(
    options: &[T],
    selected: &T,
    on_select: impl Fn(T) -> Message,
    palette: &UiPalette,
) -> Element<'a, Message>
where
    T: Clone + Display + PartialEq,
    Message: Clone + 'a,
{
    let mut segments = row![].spacing(SEGMENT_GAP);
    for option in options {
        let is_selected = option == selected;
        let palette = palette.clone();

        segments = segments.push(
            button(
                container(
                    iced::widget::text(option.to_string())
                        .size(SEGMENT_TEXT_SIZE)
                        .font(fonts::UI),
                )
                .height(Length::Fill)
                .align_y(Alignment::Center),
            )
            .height(SEGMENT_HEIGHT)
            .padding(Padding::from([0.0, SEGMENT_PADDING_X]))
            .on_press(on_select(option.clone()))
            .style(move |_, status| {
                let hovered = matches!(
                    status,
                    button::Status::Hovered | button::Status::Pressed
                );

                button::Style {
                    background: is_selected
                        .then(|| palette.surface_hover().into()),
                    text_color: if is_selected || hovered {
                        palette.text_strong()
                    } else {
                        palette.text_muted()
                    },
                    border: Border {
                        radius: radius::RADIUS_SM.into(),
                        ..Border::default()
                    },
                    ..button::Style::default()
                }
            }),
        );
    }

    let background = palette.surface_raised();
    container(segments)
        .padding(FRAME_PADDING)
        .style(move |_| container::Style {
            background: Some(background.into()),
            border: Border {
                radius: radius::RADIUS_MD.into(),
                ..Border::default()
            },
            ..container::Style::default()
        })
        .into()
}

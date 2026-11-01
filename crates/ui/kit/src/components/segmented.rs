//! `Segmented`: pick one of two to four short options.

use std::fmt::Display;

use iced::widget::{button, container, row};
use iced::{Alignment, Border, Element, Length, Padding};

use super::faded;
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
/// Border width of the selected segment.
const SELECTED_BORDER_WIDTH: f32 = 1.0;

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
    segments(options, |_| true, selected, on_select, palette)
}

/// Renders `options` like [`segmented`], with the options in `disabled`
/// faded and not pressable, for example the restart policies of a
/// container that is removed when it stops.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use otty_ui_kit::components::segmented::segmented_with_disabled;
/// use otty_ui_kit::palette::UiPalette;
///
/// #[derive(Clone)]
/// enum Message {
///     Picked(&'static str),
/// }
///
/// let palette = UiPalette::otty_dark();
/// let options = ["No", "On failure", "Always"];
/// let row: Element<'_, Message> = segmented_with_disabled(
///     &options,
///     &["On failure", "Always"],
///     &"No",
///     Message::Picked,
///     &palette,
/// );
/// ```
pub fn segmented_with_disabled<'a, T, Message>(
    options: &[T],
    disabled: &[T],
    selected: &T,
    on_select: impl Fn(T) -> Message,
    palette: &UiPalette,
) -> Element<'a, Message>
where
    T: Clone + Display + PartialEq,
    Message: Clone + 'a,
{
    segments(
        options,
        |option| !disabled.contains(option),
        selected,
        on_select,
        palette,
    )
}

/// Builds the segment row; options for which `is_enabled` is false are
/// faded and send nothing.
fn segments<'a, T, Message>(
    options: &[T],
    is_enabled: impl Fn(&T) -> bool,
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
        let is_option_enabled = is_enabled(option);
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
            .on_press_maybe(
                is_option_enabled.then(|| on_select(option.clone())),
            )
            .style(move |_, status| {
                segment_style(&palette, is_selected, is_option_enabled, status)
            }),
        );
    }

    let background = palette.surface_chrome();
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

/// The look of one segment: the selected one is a raised card with a
/// border, so it stands out from the darker frame in both themes; a
/// disabled one is faded.
fn segment_style(
    palette: &UiPalette,
    is_selected: bool,
    is_enabled: bool,
    status: button::Status,
) -> button::Style {
    let hovered =
        matches!(status, button::Status::Hovered | button::Status::Pressed);

    let border = if is_selected {
        Border {
            color: palette.line(),
            width: SELECTED_BORDER_WIDTH,
            radius: radius::RADIUS_SM.into(),
        }
    } else {
        Border {
            radius: radius::RADIUS_SM.into(),
            ..Border::default()
        }
    };
    let text_color = if is_selected || (hovered && is_enabled) {
        palette.text_strong()
    } else {
        palette.text_muted()
    };

    button::Style {
        background: is_selected.then(|| palette.surface_overlay().into()),
        text_color: if is_enabled {
            text_color
        } else {
            faded(text_color)
        },
        border,
        ..button::Style::default()
    }
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;

    use super::*;

    const RESTARTS: [&str; 3] = ["No", "On failure", "Always"];

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Message {
        Picked(&'static str),
    }

    #[test]
    fn given_a_disabled_option_when_pressed_then_nothing_is_sent() {
        for palette in [UiPalette::otty_dark(), UiPalette::otty_light()] {
            let row: Element<'_, Message> = segmented_with_disabled(
                &RESTARTS,
                &["On failure", "Always"],
                &"No",
                Message::Picked,
                &palette,
            );
            let mut ui = simulator(row);

            ui.click("Always").expect("option should be visible");

            assert_eq!(ui.into_messages().count(), 0);
        }
    }

    #[test]
    fn given_an_enabled_option_when_pressed_then_it_is_picked() {
        let palette = UiPalette::otty_dark();
        let row: Element<'_, Message> = segmented_with_disabled(
            &RESTARTS,
            &["Always"],
            &"No",
            Message::Picked,
            &palette,
        );
        let mut ui = simulator(row);

        ui.click("On failure").expect("option should be visible");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Picked("On failure")]);
    }
}

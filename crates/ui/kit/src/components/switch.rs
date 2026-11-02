//! `Switch`: on or off setting that applies right away.

use iced::widget::{button, container, row};
use iced::{Alignment, Border, Element, Length, Padding, border};

use super::ellipsis_text::EllipsisText;
use crate::palette::UiPalette;
use crate::{fonts, spacing, typography};

/// Track width (`width: 34px`).
const TRACK_WIDTH: f32 = 34.0;
/// Track height (`height: 20px`).
const TRACK_HEIGHT: f32 = 20.0;
/// Knob diameter (`width: 14px`).
const KNOB_SIZE: f32 = 14.0;
/// Gap between track edge and knob (`top: 3px; left: 3px`).
const KNOB_INSET: f32 = 3.0;

/// Switch with an optional label. Without a message it is not interactive.
/// The label stays on one line; without room it ends with "…".
///
/// # Example
///
/// ```
/// use iced::Element;
/// use otty_ui_kit::components::switch::Switch;
/// use otty_ui_kit::palette::UiPalette;
///
/// #[derive(Clone)]
/// enum Message {
///     Toggled(bool),
/// }
///
/// let palette = UiPalette::otty_dark();
/// let switch: Element<'_, Message> = Switch::new(true, &palette)
///     .label("Show command blocks")
///     .on_toggle(Message::Toggled)
///     .into();
/// ```
pub struct Switch<'a, Message> {
    is_on: bool,
    palette: UiPalette,
    label: Option<String>,
    on_toggle: Option<Box<dyn Fn(bool) -> Message + 'a>>,
}

impl<'a, Message> Switch<'a, Message> {
    /// Returns the width the switch takes when its label is shown in full, so
    /// a caller can decide what fits on one line.
    pub fn width(&self) -> f32 {
        match &self.label {
            Some(label) => {
                TRACK_WIDTH
                    + spacing::SPACE_3
                    + label_text(label, &self.palette).full_width()
            },
            None => TRACK_WIDTH,
        }
    }

    /// Creates a switch in the given state.
    pub fn new(is_on: bool, palette: &UiPalette) -> Self {
        Self {
            is_on,
            palette: palette.clone(),
            label: None,
            on_toggle: None,
        }
    }

    /// Sets the label that says what is on.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets the function that maps the new state to a message.
    pub fn on_toggle(
        mut self,
        on_toggle: impl Fn(bool) -> Message + 'a,
    ) -> Self {
        self.on_toggle = Some(Box::new(on_toggle));
        self
    }
}

impl<'a, Message: Clone + 'a> From<Switch<'a, Message>>
    for Element<'a, Message>
{
    fn from(value: Switch<'a, Message>) -> Self {
        let palette = value.palette;
        let is_on = value.is_on;

        let track_color = if is_on {
            palette.accent()
        } else {
            palette.surface_hover()
        };
        let knob_color = if is_on {
            palette.on_accent()
        } else {
            palette.text_muted()
        };
        let knob =
            container("")
                .width(KNOB_SIZE)
                .height(KNOB_SIZE)
                .style(move |_| container::Style {
                    background: Some(knob_color.into()),
                    border: Border {
                        radius: border::radius(KNOB_SIZE / 2.0),
                        ..Border::default()
                    },
                    ..container::Style::default()
                });
        let track = container(knob)
            .width(TRACK_WIDTH)
            .height(TRACK_HEIGHT)
            .padding(KNOB_INSET)
            .align_x(if is_on {
                Alignment::End
            } else {
                Alignment::Start
            })
            .align_y(Alignment::Center)
            .style(move |_| container::Style {
                background: Some(track_color.into()),
                border: Border {
                    radius: border::radius(TRACK_HEIGHT / 2.0),
                    ..Border::default()
                },
                ..container::Style::default()
            });

        let mut content = row![track]
            .spacing(spacing::SPACE_3)
            .align_y(Alignment::Center);
        if let Some(label) = value.label {
            content = content.push(label_text(&label, &palette));
        }

        let message = value.on_toggle.map(|on_toggle| on_toggle(!is_on));
        button(content)
            .padding(Padding::ZERO)
            .width(Length::Shrink)
            .on_press_maybe(message)
            .style(|_, _| button::Style::default())
            .into()
    }
}

/// The label on one line, as wide as its text: when the room runs out it ends
/// with "…" instead of wrapping.
fn label_text(label: &str, palette: &UiPalette) -> EllipsisText {
    EllipsisText::new(
        label,
        typography::BODY.size(),
        fonts::UI,
        palette.text_body(),
    )
    .shrink(0.0)
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;

    use super::*;

    #[test]
    fn given_switch_on_when_label_pressed_then_it_asks_to_turn_off() {
        for palette in [UiPalette::otty_dark(), UiPalette::otty_light()] {
            let view: Element<'_, bool> = Switch::new(true, &palette)
                .label("Follow")
                .on_toggle(|is_on| is_on)
                .into();
            let mut ui = simulator(view);

            ui.click("Follow").expect("label should show");

            assert_eq!(ui.into_messages().collect::<Vec<_>>(), vec![false]);
        }
    }

    #[test]
    fn given_switch_off_when_label_pressed_then_it_asks_to_turn_on() {
        let palette = UiPalette::otty_dark();
        let view: Element<'_, bool> = Switch::new(false, &palette)
            .label("Follow")
            .on_toggle(|is_on| is_on)
            .into();
        let mut ui = simulator(view);

        ui.click("Follow").expect("label should show");

        assert_eq!(ui.into_messages().collect::<Vec<_>>(), vec![true]);
    }

    #[test]
    fn given_switch_without_message_when_pressed_then_nothing_is_sent() {
        let palette = UiPalette::otty_dark();
        let view: Element<'_, bool> =
            Switch::new(true, &palette).label("Follow").into();
        let mut ui = simulator(view);

        ui.click("Follow").expect("label should show");

        assert!(ui.into_messages().next().is_none());
    }

    /// Bounds of `label` in a switch placed in a box `width` wide.
    fn label_bounds(label: &'static str, width: f32) -> iced::Rectangle {
        let palette = UiPalette::otty_dark();
        let switch: Element<'_, bool> =
            Switch::new(true, &palette).label(label).into();
        let mut ui = simulator(container(switch).width(width));

        ui.find(label).expect("label should show").bounds()
    }

    #[test]
    fn given_long_label_in_a_narrow_box_when_laid_out_then_it_keeps_one_line() {
        const LONG: &str = "Allow the container to read this folder";
        const WIDTH: f32 = 120.0;

        let one_line = label_bounds(LONG, 2000.0);
        let narrow = label_bounds(LONG, WIDTH);

        assert!(
            (narrow.height - one_line.height).abs() < 0.5,
            "{narrow:?} is one line tall like {one_line:?}"
        );
        assert!(
            narrow.x + narrow.width <= WIDTH + 0.5,
            "{narrow:?} stays inside the box"
        );
    }

    #[test]
    fn given_long_label_in_a_narrow_box_when_pressed_then_it_still_asks() {
        const LONG: &str = "Allow the container to read this folder";

        let palette = UiPalette::otty_dark();
        let switch: Element<'_, bool> = Switch::new(false, &palette)
            .label(LONG)
            .on_toggle(|is_on| is_on)
            .into();
        let mut ui = simulator(container(switch).width(120));

        ui.click(LONG).expect("label should show");

        assert_eq!(ui.into_messages().collect::<Vec<_>>(), vec![true]);
    }

    #[test]
    fn given_label_when_width_asked_then_it_is_the_room_the_switch_takes() {
        let palette = UiPalette::otty_dark();
        let switch: Switch<'_, bool> =
            Switch::new(true, &palette).label("Show empty");

        let label = label_bounds("Show empty", 2000.0);

        assert!(
            (switch.width() - (label.x + label.width)).abs() < 0.5,
            "{} ends where {label:?} ends",
            switch.width()
        );
    }

    #[test]
    fn given_no_label_when_width_asked_then_it_is_the_track() {
        let palette = UiPalette::otty_dark();
        let switch: Switch<'_, bool> = Switch::new(true, &palette);

        assert_eq!(switch.width(), TRACK_WIDTH);
    }
}

//! `InspectorTabs`: underlined text tabs above the inspector body.

use std::fmt::Display;

use iced::advanced::text::{self as core_text, Paragraph as _};
use iced::alignment::Vertical;
use iced::widget::text::{LineHeight, Shaping, Wrapping};
use iced::widget::{Space, button, column, container, responsive, row};
use iced::{
    Alignment, Color, Element, Length, Padding, Pixels, Renderer, Size,
};

use super::divider;
use super::select::select_owned;
use crate::palette::UiPalette;
use crate::{fonts, size, spacing};

/// Paragraph type of the renderer, used to measure the labels.
type Paragraph = <Renderer as core_text::Renderer>::Paragraph;

/// Tab text size (`font-size: 12.5px`).
const TAB_TEXT_SIZE: f32 = 12.5;
/// Tab horizontal padding (`padding: 0 2px`).
const TAB_PADDING_X: f32 = 2.0;
/// Height of the line under the selected tab (`inset 0 -2px`).
const SELECTED_LINE: f32 = 2.0;

/// Renders `tabs` as one row of text tabs over a 1px line; the one equal to
/// `selected` is strong with a 2px accent line under it. Pressing a tab sends
/// `on_select(tab)`. When the row does not fit the width it is given, a kit
/// [`select`](super::select) with the same choices and the same message
/// replaces it.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use otty_ui_kit::components::inspector_tabs::inspector_tabs;
/// use otty_ui_kit::palette::UiPalette;
///
/// #[derive(Clone)]
/// enum Message {
///     Picked(&'static str),
/// }
///
/// let palette = UiPalette::otty_dark();
/// let tabs = ["Overview", "Logs", "Inspect"];
/// let row: Element<'_, Message> =
///     inspector_tabs(&tabs, &"Overview", Message::Picked, &palette);
/// ```
pub fn inspector_tabs<'a, T, Message>(
    tabs: &[T],
    selected: &T,
    on_select: impl Fn(T) -> Message + Clone + 'a,
    palette: &UiPalette,
) -> Element<'a, Message>
where
    T: Clone + Display + PartialEq + 'a,
    Message: Clone + 'a,
{
    let tabs = tabs.to_vec();
    let selected = selected.clone();
    let palette = palette.clone();

    responsive(move |available| {
        if tabs_width(&tabs) <= available.width {
            tab_row(&tabs, &selected, on_select.clone(), &palette)
        } else {
            select_owned(
                tabs.clone(),
                Some(selected.clone()),
                on_select.clone(),
                &palette,
            )
        }
    })
    .height(Length::Shrink)
    .into()
}

/// Returns the width of the tab row: the labels with their padding and the
/// gaps between them.
fn tabs_width<T: Display>(tabs: &[T]) -> f32 {
    let labels: f32 = tabs
        .iter()
        .map(|tab| label_width(&tab.to_string()) + 2.0 * TAB_PADDING_X)
        .sum();
    let gaps = tabs.len().saturating_sub(1) as f32 * spacing::SPACE_5;

    labels + gaps
}

/// Returns the width of `label` on one line in the tab font.
fn label_width(label: &str) -> f32 {
    let paragraph = Paragraph::with_text(core_text::Text {
        content: label,
        bounds: Size::new(f32::INFINITY, f32::INFINITY),
        size: Pixels(TAB_TEXT_SIZE),
        line_height: LineHeight::default(),
        font: fonts::UI,
        align_x: core_text::Alignment::Default,
        align_y: Vertical::Top,
        shaping: Shaping::default(),
        wrapping: Wrapping::None,
    });

    paragraph.min_bounds().width
}

/// Draws `tabs` as a row of text tabs over a 1px line.
fn tab_row<'a, T, Message>(
    tabs: &[T],
    selected: &T,
    on_select: impl Fn(T) -> Message,
    palette: &UiPalette,
) -> Element<'a, Message>
where
    T: Clone + Display + PartialEq,
    Message: Clone + 'a,
{
    let mut labels = row![].spacing(spacing::SPACE_5);
    for tab in tabs {
        let is_selected = tab == selected;
        let palette = palette.clone();
        let line = if is_selected {
            palette.accent()
        } else {
            Color::TRANSPARENT
        };

        labels = labels.push(
            button(column![
                container(
                    iced::widget::text(tab.to_string())
                        .size(TAB_TEXT_SIZE)
                        .font(fonts::UI),
                )
                .height(Length::Fill)
                .align_y(Alignment::Center),
                // As wide as the label, not `Fill`, so the tab takes only
                // the room of its label and the tabs sit side by side.
                container(Space::new())
                    .width(label_width(&tab.to_string()))
                    .height(SELECTED_LINE)
                    .style(move |_| container::Style {
                        background: Some(line.into()),
                        ..container::Style::default()
                    }),
            ])
            .height(size::INSPECTOR_TAB)
            .padding(Padding::from([0.0, TAB_PADDING_X]))
            .on_press(on_select(tab.clone()))
            .style(move |_, status| {
                let hovered = matches!(
                    status,
                    button::Status::Hovered | button::Status::Pressed
                );

                button::Style {
                    text_color: if is_selected || hovered {
                        palette.text_strong()
                    } else {
                        palette.text_muted()
                    },
                    ..button::Style::default()
                }
            }),
        );
    }

    column![labels, divider::horizontal(palette)].into()
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;

    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Message {
        Picked(&'static str),
    }

    const TABS: [&str; 3] = ["Overview", "Logs", "Inspect"];

    #[test]
    fn given_tabs_when_rendered_then_every_label_is_shown() {
        let palette = UiPalette::otty_dark();
        let tabs: Element<'_, Message> =
            inspector_tabs(&TABS, &"Overview", Message::Picked, &palette);
        let mut ui = simulator(tabs);

        for label in TABS {
            assert!(ui.find(label).is_ok(), "{label} should be shown");
        }
    }

    #[test]
    fn given_tabs_when_one_pressed_then_its_message_is_sent() {
        let palette = UiPalette::otty_light();
        let tabs: Element<'_, Message> =
            inspector_tabs(&TABS, &"Overview", Message::Picked, &palette);
        let mut ui = simulator(tabs);

        ui.click("Logs").expect("tab should be shown");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Picked("Logs")]);
    }

    /// Width that holds the three tabs in the tests.
    const WIDE: f32 = 400.0;
    /// Width that holds none of the tab rows but a select.
    const NARROW: f32 = 120.0;
    /// Middle of the second menu row under the 34px field.
    const SECOND_ROW_Y: f32 = 34.0 + 34.0 + 17.0;

    fn tabs_in(width: f32, palette: &UiPalette) -> Element<'_, Message> {
        iced::widget::container(inspector_tabs(
            &TABS,
            &"Overview",
            Message::Picked,
            palette,
        ))
        .width(width)
        .into()
    }

    #[test]
    fn given_enough_width_when_rendered_then_the_tabs_are_a_row_of_labels() {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(tabs_in(WIDE, &palette));

        for label in TABS {
            assert!(ui.find(label).is_ok(), "{label} should be a tab");
        }
        ui.click("Logs").expect("tab should be shown");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Picked("Logs")]);
    }

    #[test]
    fn given_too_little_width_when_rendered_then_a_select_replaces_the_tabs() {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(tabs_in(NARROW, &palette));

        for label in ["Logs", "Inspect"] {
            assert!(
                ui.find(label).is_err(),
                "{label} should not be a tab when they do not fit"
            );
        }

        // The pick list draws its text itself, so it is pressed by
        // position: the 34px field, then the second 34px menu row under it.
        ui.point_at(iced::Point::new(20.0, 17.0));
        let _ = ui.simulate(iced_test::simulator::click());
        let row = iced::Point::new(20.0, SECOND_ROW_Y);
        ui.point_at(row);
        let _ = ui.simulate([iced::Event::Mouse(
            iced::mouse::Event::CursorMoved { position: row },
        )]);
        let _ = ui.simulate(iced_test::simulator::click());

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Picked("Logs")]);
    }

    #[test]
    fn given_tabs_when_rendered_then_they_sit_side_by_side_from_the_left() {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(tabs_in(WIDE, &palette));

        let overview = ui.find("Overview").expect("tab").bounds();
        let logs = ui.find("Logs").expect("tab").bounds();

        assert_eq!(overview.x, TAB_PADDING_X);
        assert!(
            (logs.x
                - (overview.x
                    + overview.width
                    + 2.0 * TAB_PADDING_X
                    + spacing::SPACE_5))
                .abs()
                < 0.5,
            "Logs {logs:?} should follow Overview {overview:?} after the gap"
        );
    }

    #[test]
    fn given_a_width_that_just_fits_the_tabs_when_rendered_then_they_stay_tabs()
    {
        let palette = UiPalette::otty_dark();
        let mut wide = simulator(tabs_in(WIDE, &palette));
        let labels = TABS.map(|label| wide.find(label).expect("tab").bounds());
        let last = labels[2];
        let needed = last.x + last.width + TAB_PADDING_X;

        let mut exact = simulator(tabs_in(needed + 0.5, &palette));
        let mut short = simulator(tabs_in(needed - 8.0, &palette));

        assert!(exact.find("Inspect").is_ok());
        assert!(short.find("Inspect").is_err());
    }

    #[test]
    fn given_selected_tab_when_pressed_then_message_is_still_sent() {
        let palette = UiPalette::otty_dark();
        let tabs: Element<'_, Message> =
            inspector_tabs(&TABS, &"Inspect", Message::Picked, &palette);
        let mut ui = simulator(tabs);

        ui.click("Inspect").expect("tab should be shown");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Picked("Inspect")]);
    }
}

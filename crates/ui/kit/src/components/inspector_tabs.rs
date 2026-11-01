//! `InspectorTabs`: underlined text tabs above the inspector body.

use std::fmt::Display;

use iced::widget::{Space, button, column, container, row};
use iced::{Alignment, Color, Element, Length, Padding};

use super::divider;
use crate::palette::UiPalette;
use crate::{fonts, size, spacing};

/// Tab text size (`font-size: 12.5px`).
const TAB_TEXT_SIZE: f32 = 12.5;
/// Tab horizontal padding (`padding: 0 2px`).
const TAB_PADDING_X: f32 = 2.0;
/// Height of the line under the selected tab (`inset 0 -2px`).
const SELECTED_LINE: f32 = 2.0;

/// Renders `tabs` as one row of text tabs over a 1px line; the one equal to
/// `selected` is strong with a 2px accent line under it. Pressing a tab sends
/// `on_select(tab)`.
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
                container(Space::new())
                    .width(Length::Fill)
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

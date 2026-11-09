//! `NotificationList`: the popup list of past notifications.

use iced::widget::text::Wrapping;
use iced::widget::{column, container, row, scrollable, svg, text};
use iced::{Alignment, Border, Element, Length, Padding};

use super::button::{Button, ButtonKind, ButtonSize};
use super::{menu, scrollbar};
use crate::palette::UiPalette;
use crate::{fonts, radius, spacing};

/// Width of the list.
const LIST_WIDTH: f32 = 360.0;
/// Height above which the rows scroll.
const LIST_MAX_HEIGHT: f32 = 420.0;
/// Padding around the header and the rows (`padding: 5px`, as in `Menu`).
const LIST_PADDING: f32 = menu::MENU_PADDING;
/// Text size of the title.
const TITLE_SIZE: f32 = 13.0;
/// Text size of a notification.
const TEXT_SIZE: f32 = 12.5;
/// Text size of the age under a notification.
const AGE_SIZE: f32 = 12.0;
/// Edge of the error icon of a row.
const ICON_SIZE: f32 = 14.0;
/// Gap between the icon and the text of a row.
const ROW_GAP: f32 = 10.0;
/// Padding of a row and of the header (`padding: 0 10px`, as menu items).
const ROW_PADDING_X: f32 = 10.0;

/// One past notification: its text and how long ago it came.
struct Row {
    text: String,
    age: String,
}

/// Popup list of past notifications: a header with the title and a Clear
/// button, then one row per notification with the error icon, its text and
/// a muted age such as "3 min ago". The caller adds the rows newest first.
/// Many rows scroll; no rows show the empty text. Rows take no presses.
///
/// Components never bundle icons, so the caller passes the error icon.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use iced::widget::svg;
/// use otty_ui_kit::components::notification_list::NotificationList;
/// use otty_ui_kit::palette::UiPalette;
///
/// #[derive(Clone)]
/// enum Message {
///     Clear,
/// }
///
/// let palette = UiPalette::otty_dark();
/// let icon = svg::Handle::from_memory(&b"<svg/>"[..]);
/// let list: Element<'_, Message> =
///     NotificationList::new("Notifications", icon, &palette)
///         .clear("Clear", Some(Message::Clear))
///         .empty("No notifications")
///         .row("Could not save the pinned modes: disk full", "just now")
///         .row("Could not stop web-1: busy", "3 min ago")
///         .into();
/// ```
pub struct NotificationList<Message> {
    title: String,
    icon: svg::Handle,
    palette: UiPalette,
    clear: Option<(String, Option<Message>)>,
    empty: String,
    rows: Vec<Row>,
}

impl<Message> NotificationList<Message> {
    /// Creates an empty list with a `title` and the error `icon` its rows
    /// show.
    pub fn new(
        title: impl Into<String>,
        icon: svg::Handle,
        palette: &UiPalette,
    ) -> Self {
        Self {
            title: title.into(),
            icon,
            palette: palette.clone(),
            clear: None,
            empty: String::new(),
            rows: Vec::new(),
        }
    }

    /// Shows a Clear button with `label` in the header. It sends `message`,
    /// and without one it is disabled, as when there is nothing to clear.
    pub fn clear(
        mut self,
        label: impl Into<String>,
        message: Option<Message>,
    ) -> Self {
        self.clear = Some((label.into(), message));
        self
    }

    /// Sets the text shown while the list has no rows.
    pub fn empty(mut self, text: impl Into<String>) -> Self {
        self.empty = text.into();
        self
    }

    /// Appends a row with the notification `text` and its `age`, such as
    /// "just now".
    pub fn row(
        mut self,
        text: impl Into<String>,
        age: impl Into<String>,
    ) -> Self {
        self.rows.push(Row {
            text: text.into(),
            age: age.into(),
        });
        self
    }
}

impl<'a, Message: Clone + 'a> From<NotificationList<Message>>
    for Element<'a, Message>
{
    fn from(value: NotificationList<Message>) -> Self {
        let palette = value.palette;

        let header = header_view(value.title, value.clear, &palette);
        let body = if value.rows.is_empty() {
            empty_view(value.empty, &palette)
        } else {
            rows_view(value.rows, &value.icon, &palette)
        };

        let background = palette.surface_overlay();
        let shadow = palette.shadow_popover();
        container(column![header, body].width(Length::Fill))
            .width(LIST_WIDTH)
            .padding(LIST_PADDING)
            .style(move |_| container::Style {
                background: Some(background.into()),
                border: Border {
                    radius: radius::RADIUS_MD.into(),
                    ..Border::default()
                },
                shadow,
                ..container::Style::default()
            })
            .into()
    }
}

/// Builds the header: the title, and the Clear button at the right.
fn header_view<'a, Message: Clone + 'a>(
    title: String,
    clear: Option<(String, Option<Message>)>,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let mut line = row![
        text(title)
            .size(TITLE_SIZE)
            .font(fonts::UI_SEMIBOLD)
            .color(palette.text_strong())
            .width(Length::Fill)
    ]
    .align_y(Alignment::Center);
    if let Some((label, message)) = clear {
        let button = Button::new(label, palette)
            .kind(ButtonKind::Ghost)
            .size(ButtonSize::Small);
        line = line.push(match message {
            Some(message) => button.on_press(message),
            None => button,
        });
    }

    container(line)
        .padding(Padding {
            top: 0.0,
            right: 0.0,
            bottom: spacing::SPACE_1,
            left: ROW_PADDING_X,
        })
        .into()
}

/// Builds the muted line shown while there are no rows.
fn empty_view<'a, Message: 'a>(
    empty: String,
    palette: &UiPalette,
) -> Element<'a, Message> {
    container(
        text(empty)
            .size(TEXT_SIZE)
            .font(fonts::UI)
            .color(palette.text_muted())
            .wrapping(Wrapping::Word),
    )
    .padding(Padding::from([spacing::SPACE_3, ROW_PADDING_X]))
    .into()
}

/// Builds the rows in a scroller that stops growing at
/// [`LIST_MAX_HEIGHT`].
fn rows_view<'a, Message: 'a>(
    rows: Vec<Row>,
    icon: &svg::Handle,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let mut list = column![].width(Length::Fill);
    for notification in rows {
        list = list.push(row_view(notification, icon.clone(), palette));
    }

    container(
        scrollable(list)
            .width(Length::Fill)
            .direction(scrollbar::vertical())
            .style(scrollbar::style(palette)),
    )
    .max_height(LIST_MAX_HEIGHT)
    .into()
}

/// Builds one row: the error icon, then the text with its age under it.
/// Long text wraps.
fn row_view<'a, Message: 'a>(
    notification: Row,
    icon: svg::Handle,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let icon_color = palette.status_error();
    let icon =
        svg(icon)
            .width(ICON_SIZE)
            .height(ICON_SIZE)
            .style(move |_, _| svg::Style {
                color: Some(icon_color),
            });
    let words = column![
        text(notification.text)
            .size(TEXT_SIZE)
            .font(fonts::UI)
            .color(palette.text_body())
            .wrapping(Wrapping::WordOrGlyph),
        text(notification.age)
            .size(AGE_SIZE)
            .font(fonts::UI)
            .color(palette.text_muted()),
    ]
    .spacing(2.0)
    .width(Length::Fill);

    container(row![icon, words].spacing(ROW_GAP).align_y(Alignment::Start))
        .padding(Padding::from([spacing::SPACE_2, ROW_PADDING_X]))
        .into()
}

#[cfg(test)]
mod tests {
    use iced_test::selector::Candidate;
    use iced_test::{Simulator, simulator};

    use super::*;
    use crate::components::glyph;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Message {
        Clear,
    }

    fn list(rows: &[(&str, &str)]) -> NotificationList<Message> {
        let palette = UiPalette::otty_dark();
        let mut list =
            NotificationList::new("Notifications", glyph::plus(), &palette)
                .clear("Clear", Some(Message::Clear))
                .empty("No notifications");
        for (text, age) in rows {
            list = list.row(*text, *age);
        }

        list
    }

    #[test]
    fn given_rows_then_each_shows_its_text_and_age_in_the_given_order() {
        let mut ui = simulator(list(&[
            ("Could not save the pinned modes: disk full", "just now"),
            ("Could not stop web-1: busy", "3 min ago"),
        ]));

        let newest = ui
            .find("Could not save the pinned modes: disk full")
            .expect("newest row should show")
            .bounds();
        let older = ui
            .find("Could not stop web-1: busy")
            .expect("older row should show")
            .bounds();

        assert!(ui.find("just now").is_ok());
        assert!(ui.find("3 min ago").is_ok());
        assert!(ui.find("Notifications").is_ok());
        assert!(ui.find("No notifications").is_err());
        assert!(newest.y < older.y);
    }

    #[test]
    fn given_no_rows_then_the_empty_text_shows() {
        let mut ui = simulator(list(&[]));

        assert!(ui.find("Notifications").is_ok());
        assert!(ui.find("No notifications").is_ok());
    }

    #[test]
    fn given_clear_with_a_message_when_pressed_then_it_is_sent() {
        let mut ui =
            simulator(list(&[("Could not stop web-1: busy", "1 h ago")]));

        ui.click("Clear").expect("Clear should show");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Clear]);
    }

    #[test]
    fn given_clear_without_a_message_when_pressed_then_nothing_is_sent() {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(
            NotificationList::<Message>::new(
                "Notifications",
                glyph::plus(),
                &palette,
            )
            .clear("Clear", None)
            .empty("No notifications"),
        );

        ui.click("Clear").expect("Clear should show");

        assert_eq!(ui.into_messages().count(), 0);
    }

    #[test]
    fn given_a_row_when_pressed_then_nothing_is_sent() {
        let mut ui =
            simulator(list(&[("Could not stop web-1: busy", "1 h ago")]));

        ui.click("Could not stop web-1: busy")
            .expect("row should show");

        assert_eq!(ui.into_messages().count(), 0);
    }

    /// The scroller of the rows: where it is and how tall its rows are.
    fn rows_scroller(
        ui: &mut Simulator<'_, Message>,
    ) -> (iced::Rectangle, iced::Rectangle) {
        ui.find(|candidate: Candidate<'_>| match candidate {
            Candidate::Scrollable {
                bounds,
                content_bounds,
                ..
            } => Some((bounds, content_bounds)),
            _ => None,
        })
        .expect("the rows should scroll")
    }

    #[test]
    fn given_many_rows_then_the_list_keeps_its_height_and_the_rows_scroll() {
        let texts: Vec<String> = (0..40)
            .map(|index| format!("Could not stop web-{index}: busy"))
            .collect();
        let rows: Vec<(&str, &str)> = texts
            .iter()
            .map(|text| (text.as_str(), "just now"))
            .collect();
        let mut ui = simulator(list(&rows));

        let (bounds, content) = rows_scroller(&mut ui);

        assert!(bounds.height <= LIST_MAX_HEIGHT, "{bounds:?}");
        assert!(content.height > bounds.height, "{content:?}");
    }

    #[test]
    fn given_a_long_text_then_it_wraps_inside_the_list() {
        let long = "Could not save the pinned modes: the configuration \
                    folder is on a disk that is full, so nothing was written";
        let mut ui = simulator(list(&[(long, "just now")]));

        let text = ui.find(long).expect("row should show").bounds();
        let title = ui.find("Notifications").expect("title").bounds();

        assert!(text.x + text.width <= LIST_WIDTH, "{text:?}");
        assert!(text.height > title.height * 1.5, "{text:?}");
    }

    #[test]
    fn given_list_then_it_is_its_fixed_width() {
        let mut ui = simulator(list(&[]));

        let clear = ui.find("Clear").expect("Clear should show").bounds();

        // Clear ends the header, near the right edge of the list.
        let right = clear.x + clear.width;
        assert!(
            right > LIST_WIDTH - 40.0 && right <= LIST_WIDTH,
            "{clear:?}"
        );
    }
}

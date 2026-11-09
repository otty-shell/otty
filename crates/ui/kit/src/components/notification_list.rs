//! `NotificationList`: the popup list of past notifications.

use iced::widget::text::Wrapping;
use iced::widget::{column, container, row, scrollable, svg, text};
use iced::{Alignment, Border, Element, Length, Padding};

use super::icon_button::{IconButton, IconButtonSize};
use super::icon_hover::IconHover;
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
/// Right padding of the header: the broom has room around its icon.
const HEADER_PADDING_RIGHT: f32 = spacing::SPACE_1;

/// One past notification: its text and how long ago it came.
struct Row {
    text: String,
    age: String,
}

/// The Clear all button: its icon, its tooltip and its message, if any.
struct Clear<Message> {
    icon: svg::Handle,
    label: String,
    on_press: Option<Message>,
}

/// Popup list of past notifications: a header with the title and a Clear
/// all icon button, then one row per notification with the error icon, its
/// text and a muted age such as "3 min ago". The caller adds the rows newest
/// first. A row lights up while the pointer is over it, but takes no
/// presses. Many rows scroll; no rows show the empty text.
///
/// Components never bundle icons, so the caller passes the error icon and
/// the Clear all icon, such as a broom.
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
/// let error = svg::Handle::from_memory(&b"<svg/>"[..]);
/// let broom = svg::Handle::from_memory(&b"<svg/>"[..]);
/// let list: Element<'_, Message> =
///     NotificationList::new("Notifications", error, &palette)
///         .clear(broom, "Clear all", Some(Message::Clear))
///         .empty("No notifications")
///         .row("Could not save the pinned modes: disk full", "just now")
///         .row("Could not stop web-1: busy", "3 min ago")
///         .into();
/// ```
pub struct NotificationList<Message> {
    title: String,
    icon: svg::Handle,
    palette: UiPalette,
    clear: Option<Clear<Message>>,
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

    /// Shows a Clear all icon button with `icon` in the header; `label` is
    /// its tooltip, such as "Clear all". It sends `message`, and without one
    /// it is disabled, as when there is nothing to clear.
    pub fn clear(
        mut self,
        icon: svg::Handle,
        label: impl Into<String>,
        message: Option<Message>,
    ) -> Self {
        self.clear = Some(Clear {
            icon,
            label: label.into(),
            on_press: message,
        });
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

/// Builds the header: the title, and the Clear all icon button at the
/// right.
fn header_view<'a, Message: Clone + 'a>(
    title: String,
    clear: Option<Clear<Message>>,
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
    if let Some(clear) = clear {
        line = line.push(
            IconButton::new(clear.icon, clear.label, palette)
                .size(IconButtonSize::Small)
                .on_press_maybe(clear.on_press),
        );
    }

    container(line)
        .padding(Padding {
            top: 0.0,
            right: HEADER_PADDING_RIGHT,
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
/// Long text wraps. The row lights up while the pointer is over it.
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

    let pointer = IconHover::new();
    let lit_row =
        container(row![icon, words].spacing(ROW_GAP).align_y(Alignment::Start))
            .padding(Padding::from([spacing::SPACE_2, ROW_PADDING_X]))
            .style(pointer.container_style(
                row_style(palette, false),
                row_style(palette, true),
            ));

    pointer.watch(lit_row)
}

/// Style of a row: the `surface-hover` of menu items while the pointer is
/// over it, no background otherwise.
fn row_style(palette: &UiPalette, is_lit: bool) -> container::Style {
    container::Style {
        background: is_lit.then(|| palette.surface_hover().into()),
        border: Border {
            radius: radius::RADIUS_XS.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

#[cfg(test)]
mod tests {
    use iced::{Event, Point, mouse};
    use iced_test::selector::Candidate;
    use iced_test::{Simulator, simulator};

    use super::*;
    use crate::components::glyph;
    use crate::size;
    use crate::test_support::TempDir;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Message {
        Clear,
    }

    fn list(rows: &[(&str, &str)]) -> NotificationList<Message> {
        let palette = UiPalette::otty_dark();
        let mut list =
            NotificationList::new("Notifications", glyph::plus(), &palette)
                .clear(glyph::plus(), "Clear all", Some(Message::Clear))
                .empty("No notifications");
        for (text, age) in rows {
            list = list.row(*text, *age);
        }

        list
    }

    /// Horizontal center of the broom when the list sits at the origin.
    fn broom_x() -> f32 {
        LIST_WIDTH
            - LIST_PADDING
            - HEADER_PADDING_RIGHT
            - size::CONTROL_SM / 2.0
    }

    /// Presses the broom, which is centered on the title line.
    fn press_broom(ui: &mut Simulator<'_, Message>) {
        let title = ui.find("Notifications").expect("title").bounds();

        ui.point_at(Point::new(broom_x(), title.center_y()));
        let _ = ui.simulate(simulator::click());
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
    fn given_clear_with_a_message_when_the_broom_is_pressed_then_it_is_sent() {
        let mut ui =
            simulator(list(&[("Could not stop web-1: busy", "1 h ago")]));

        press_broom(&mut ui);

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Clear]);
    }

    #[test]
    fn given_clear_without_a_message_when_the_broom_is_pressed_then_nothing_is_sent()
     {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(
            NotificationList::<Message>::new(
                "Notifications",
                glyph::plus(),
                &palette,
            )
            .clear(glyph::plus(), "Clear all", None)
            .empty("No notifications"),
        );

        press_broom(&mut ui);

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

    /// Two rows, the first with a long text that wraps, for the pictures
    /// of the hover tests.
    fn hover_list() -> NotificationList<Message> {
        list(&[
            ("Could not save the pinned modes: disk full", "just now"),
            ("Could not stop web-1: busy", "3 min ago"),
        ])
    }

    /// Spots for the pointer: on the text of the first row, on the left
    /// padding of the first row, on the text of the second row, and on
    /// the title.
    fn pointer_spots() -> [Point; 4] {
        let mut ui = simulator(hover_list());
        let first = ui
            .find("Could not save the pinned modes: disk full")
            .expect("first row should show")
            .bounds();
        let second = ui
            .find("Could not stop web-1: busy")
            .expect("second row should show")
            .bounds();
        let title = ui.find("Notifications").expect("title").bounds();

        [
            Point::new(first.x + 4.0, first.center_y()),
            Point::new(LIST_PADDING + 2.0, first.center_y()),
            Point::new(second.x + 4.0, second.center_y()),
            Point::new(title.x + 4.0, title.center_y()),
        ]
    }

    /// Draws the hover list with the pointer at `pointer`, or with no
    /// pointer, and checks it against the picture saved in `reference`;
    /// the first call saves the picture.
    fn drawn_list_matches(
        pointer: Option<Point>,
        reference: &std::path::Path,
    ) -> bool {
        let mut ui = simulator(hover_list());
        if let Some(position) = pointer {
            ui.point_at(position);
            let _ = ui.simulate([Event::Mouse(mouse::Event::CursorMoved {
                position,
            })]);
        }

        ui.snapshot(&iced::Theme::Dark)
            .expect("list should draw")
            .matches_hash(reference)
            .expect("reference should be readable")
    }

    /// A row lights up wherever the pointer is on it, the padding too, and
    /// only that row: with the pointer on another row, on the title or
    /// away, it is not lit.
    #[test]
    fn given_a_row_when_the_pointer_is_over_it_then_only_that_row_lights_up() {
        let folder = TempDir::new("notification-list");
        let lit = folder.path().join("first-row-lit");
        let idle = folder.path().join("idle");
        let [on_text, on_padding, on_second_row, on_title] = pointer_spots();

        assert!(drawn_list_matches(Some(on_text), &lit));
        assert!(drawn_list_matches(Some(on_padding), &lit));
        assert!(!drawn_list_matches(Some(on_second_row), &lit));
        assert!(drawn_list_matches(None, &idle));
        assert!(drawn_list_matches(Some(on_title), &idle));
        assert!(!drawn_list_matches(None, &lit));
    }

    /// A lit row has the hover surface of menu items, as the other rows of
    /// a popup do; an idle row has no background of its own.
    #[test]
    fn given_a_row_then_it_has_the_menu_hover_surface_only_while_lit() {
        for palette in [UiPalette::otty_dark(), UiPalette::otty_light()] {
            let lit = row_style(&palette, true);
            let idle = row_style(&palette, false);

            assert_eq!(lit.background, Some(palette.surface_hover().into()));
            assert_eq!(lit.border.radius, radius::RADIUS_XS.into());
            assert_eq!(idle.background, None);
        }
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

    /// The title fills the header up to the broom, which ends near the
    /// right edge of the fixed-width list.
    #[test]
    fn given_list_then_it_is_its_fixed_width() {
        let mut ui = simulator(list(&[]));

        let title = ui.find("Notifications").expect("title").bounds();

        let broom_left = broom_x() - size::CONTROL_SM / 2.0;
        let right = title.x + title.width;
        assert!((right - broom_left).abs() < 1.0, "{title:?}");
    }
}

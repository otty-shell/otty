//! `SessionRow`: one session under a group label.

use iced::widget::text::Wrapping;
use iced::widget::{button, container, hover, mouse_area, row, space, text};
use iced::{Alignment, Border, Color, Element, Length, Padding};

use super::glyph;
use super::icon_button::{IconButton, IconButtonSize};
use crate::palette::UiPalette;
use crate::{fonts, radius, size, spacing, typography};

/// Gap between title, state and close (`gap: 8px`).
const GAP: f32 = spacing::SPACE_3;
/// Right padding of the row (`padding: 0 8px`).
const PADDING_RIGHT: f32 = spacing::SPACE_3;

/// Color of the state word of a [`SessionRow`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionTone {
    /// `text-muted`: "connecting", "editor".
    Muted,
    /// `status-warn`: the session waits for the user.
    Waiting,
    /// `status-ok`: the work is done.
    Done,
    /// `status-error`: the session failed.
    Failed,
}

impl SessionTone {
    /// Returns the color of this tone in `palette`.
    pub const fn color(self, palette: &UiPalette) -> Color {
        match self {
            Self::Muted => palette.text_muted(),
            Self::Waiting => palette.status_warn(),
            Self::Done => palette.status_ok(),
            Self::Failed => palette.status_error(),
        }
    }
}

/// Session row, indented to line up under its [`SessionGroup`] name.
///
/// The title and the muted folder after it are cut at the row edge: `iced`
/// has no text ellipsis. The close
/// cross is set with `on_close` and shows only while the pointer is over the
/// row. The row always keeps room for it, so nothing shifts when it appears.
///
/// [`SessionGroup`]: super::session_group::SessionGroup
///
/// # Example
///
/// ```
/// use iced::Element;
/// use otty_ui_kit::components::session_row::{SessionRow, SessionTone};
/// use otty_ui_kit::palette::UiPalette;
///
/// #[derive(Clone)]
/// enum Message {
///     Open,
/// }
///
/// let palette = UiPalette::otty_dark();
/// let row: Element<'_, Message> = SessionRow::new("claude", &palette)
///     .folder("otty")
///     .state("needs you", SessionTone::Waiting)
///     .on_press(Message::Open)
///     .into();
/// ```
pub struct SessionRow<Message> {
    title: String,
    folder: Option<String>,
    palette: UiPalette,
    is_selected: bool,
    state: Option<(String, SessionTone)>,
    on_press: Option<Message>,
    on_right_press: Option<Message>,
    on_middle_press: Option<Message>,
    on_close: Option<(Message, String)>,
}

impl<Message> SessionRow<Message> {
    /// Creates a row with `title` only.
    pub fn new(title: impl Into<String>, palette: &UiPalette) -> Self {
        Self {
            title: title.into(),
            folder: None,
            palette: palette.clone(),
            is_selected: false,
            state: None,
            on_press: None,
            on_right_press: None,
            on_middle_press: None,
            on_close: None,
        }
    }

    /// Marks the row as the selected one.
    pub fn selected(mut self, is_selected: bool) -> Self {
        self.is_selected = is_selected;
        self
    }

    /// Shows the folder the session works in after the title, muted.
    pub fn folder(mut self, folder: impl Into<String>) -> Self {
        self.folder = Some(folder.into());
        self
    }

    /// Shows a state word after the title, such as "connecting".
    pub fn state(mut self, word: impl Into<String>, tone: SessionTone) -> Self {
        self.state = Some((word.into(), tone));
        self
    }

    /// Sets the message sent on press.
    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    /// Sets the message sent on right press.
    pub fn on_right_press(mut self, message: Message) -> Self {
        self.on_right_press = Some(message);
        self
    }

    /// Sets the message sent on a middle click, for example to close the
    /// session the way a middle click closes a browser tab.
    pub fn on_middle_press(mut self, message: Message) -> Self {
        self.on_middle_press = Some(message);
        self
    }

    /// Shows a close cross that sends `message` while the row is hovered.
    /// `label` is the cross's tooltip, such as "Close session", in the
    /// app's language.
    pub fn on_close(
        mut self,
        message: Message,
        label: impl Into<String>,
    ) -> Self {
        self.on_close = Some((message, label.into()));
        self
    }
}

impl<'a, Message: Clone + 'a> From<SessionRow<Message>>
    for Element<'a, Message>
{
    fn from(value: SessionRow<Message>) -> Self {
        let palette = value.palette;
        let is_selected = value.is_selected;
        let body = typography::BODY;
        let caption = typography::CAPTION;

        let mut label = row![
            text(value.title)
                .size(body.size())
                .font(fonts::UI)
                .wrapping(Wrapping::None),
        ]
        .spacing(GAP)
        .align_y(Alignment::Center);
        if let Some(folder) = value.folder {
            label = label.push(
                text(folder)
                    .size(caption.size())
                    .font(fonts::MONO)
                    .color(palette.text_muted())
                    .wrapping(Wrapping::None),
            );
        }

        let mut content =
            row![container(label).width(Length::Fill).clip(true),]
                .spacing(GAP)
                .height(Length::Fill)
                .align_y(Alignment::Center);
        if let Some((word, tone)) = value.state {
            content = content.push(
                text(word)
                    .size(caption.size())
                    .font(fonts::UI)
                    .color(tone.color(&palette)),
            );
        }
        let has_close = value.on_close.is_some();
        if has_close {
            content = content.push(space().width(size::CONTROL_SM));
        }

        // The cross is drawn over the reserved room at the right edge, so the
        // row button keeps no right padding when a cross is possible.
        let row_palette = palette.clone();
        let control = button(content)
            .width(Length::Fill)
            .height(size::ROW)
            .padding(Padding {
                top: 0.0,
                right: if has_close { 0.0 } else { PADDING_RIGHT },
                bottom: 0.0,
                left: size::SESSION_INDENT,
            })
            .on_press_maybe(value.on_press)
            .style(move |_, status| button::Style {
                background: if is_selected {
                    Some(row_palette.surface_selected().into())
                } else {
                    (status == button::Status::Hovered)
                        .then(|| row_palette.surface_raised().into())
                },
                text_color: if is_selected {
                    row_palette.text_strong()
                } else {
                    row_palette.text_body()
                },
                border: Border {
                    radius: radius::RADIUS_SM.into(),
                    ..Border::default()
                },
                ..button::Style::default()
            });

        let mut area = mouse_area(control);
        if let Some(message) = value.on_right_press {
            area = area.on_right_press(message);
        }
        if let Some(message) = value.on_middle_press {
            area = area.on_middle_press(message);
        }

        let Some((message, label)) = value.on_close else {
            return area.into();
        };

        // No hover background: on a selected row it would draw a dark square
        // behind the cross. The cross itself turns brighter instead.
        let cross = container(
            IconButton::new(glyph::cross(), label, &palette)
                .size(IconButtonSize::Small)
                .hover_background(false)
                .on_press(message),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::End)
        .align_y(Alignment::Center);

        hover(area, cross)
    }
}

#[cfg(test)]
mod tests {
    use iced::Point;
    use iced_test::simulator;

    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Message {
        Open,
        Menu,
        Close,
    }

    fn row_view() -> Element<'static, Message> {
        let palette = UiPalette::otty_dark();

        container(
            SessionRow::new("htop", &palette)
                .state("editor", SessionTone::Muted)
                .on_press(Message::Open)
                .on_right_press(Message::Menu)
                .on_close(Message::Close, "Fermer la session"),
        )
        .width(260)
        .into()
    }

    #[test]
    fn given_row_when_title_pressed_then_press_is_sent() {
        let mut ui = simulator(row_view());

        ui.click("htop").expect("title should be visible");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Open]);
    }

    /// Draws a selected row and checks it against the picture saved in
    /// `reference`; the first call saves the picture. The row is selected so
    /// that hover does not change its background: only the cross can differ.
    fn drawn_row_matches(
        with_close: bool,
        is_hovered: bool,
        reference: &std::path::Path,
    ) -> bool {
        let palette = UiPalette::otty_dark();
        let mut row = SessionRow::new("htop", &palette)
            .selected(true)
            .state("editor", SessionTone::Muted)
            .on_press(Message::Open);
        if with_close {
            row = row.on_close(Message::Close, "Close session");
        }
        let mut ui = simulator(container(row).width(260));
        if is_hovered {
            ui.point_at(Point::new(40.0, size::ROW / 2.0));
        }

        ui.snapshot(&iced::Theme::Dark)
            .expect("row should draw")
            .matches_hash(reference)
            .expect("reference should be readable")
    }

    /// Draws a row with the pointer over its close cross, whose tooltip is
    /// `label`, and checks it against the picture saved in `reference`; the
    /// first call saves the picture.
    fn hovered_cross_matches(label: &str, reference: &std::path::Path) -> bool {
        let palette = UiPalette::otty_dark();
        let row = SessionRow::new("htop", &palette)
            .selected(true)
            .on_close(Message::Close, label);
        let mut ui = simulator(container(row).width(260));
        let cross = Point::new(260.0 - 12.0, size::ROW / 2.0);
        ui.point_at(cross);
        let _ = ui.simulate([iced::Event::Mouse(
            iced::mouse::Event::CursorMoved { position: cross },
        )]);

        ui.snapshot(&iced::Theme::Dark)
            .expect("row should draw")
            .matches_hash(reference)
            .expect("reference should be readable")
    }

    fn reference_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir()
            .join(format!("otty-session-row-{}", std::process::id()))
            .join(name)
    }

    #[test]
    fn given_selected_row_without_hover_then_close_cross_is_not_drawn() {
        let reference = reference_path("hovered");

        assert!(drawn_row_matches(true, true, &reference));
        assert!(!drawn_row_matches(true, false, &reference));
        assert!(drawn_row_matches(true, true, &reference));
    }

    #[test]
    fn given_row_when_pointer_over_it_then_close_cross_is_drawn() {
        let reference = reference_path("with-close");

        assert!(drawn_row_matches(true, true, &reference));
        assert!(!drawn_row_matches(false, true, &reference));
    }

    #[test]
    fn given_close_label_when_cross_hovered_then_the_tooltip_shows_that_label()
    {
        let reference = reference_path("close-label");

        assert!(hovered_cross_matches("Close session", &reference));
        assert!(!hovered_cross_matches("Fermer la session", &reference));
        assert!(hovered_cross_matches("Close session", &reference));
    }

    #[test]
    fn given_row_without_close_then_hover_draws_no_cross() {
        let reference = reference_path("no-close");

        assert!(drawn_row_matches(false, true, &reference));
        assert!(drawn_row_matches(false, false, &reference));
    }

    #[test]
    fn given_hovered_row_when_close_pressed_then_only_close_is_sent() {
        let mut ui = simulator(row_view());

        // Move over the row first, then press the cross: the last 24px of
        // the 260px wide row.
        ui.point_at(Point::new(40.0, size::ROW / 2.0));
        ui.point_at(Point::new(260.0 - 12.0, size::ROW / 2.0));
        let _ = ui.simulate(iced_test::simulator::click());

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Close]);
    }

    #[test]
    fn given_row_when_right_pressed_then_menu_is_sent() {
        let mut ui = simulator(row_view());

        ui.point_at(Point::new(40.0, size::ROW / 2.0));
        let _ = ui.simulate([
            iced::Event::Mouse(iced::mouse::Event::ButtonPressed(
                iced::mouse::Button::Right,
            )),
            iced::Event::Mouse(iced::mouse::Event::ButtonReleased(
                iced::mouse::Button::Right,
            )),
        ]);

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Menu]);
    }

    #[test]
    fn given_row_when_middle_pressed_then_middle_press_is_sent() {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(
            container(
                SessionRow::new("htop", &palette)
                    .on_press(Message::Open)
                    .on_middle_press(Message::Close),
            )
            .width(260),
        );

        ui.point_at(Point::new(40.0, size::ROW / 2.0));
        let _ = ui.simulate([
            iced::Event::Mouse(iced::mouse::Event::ButtonPressed(
                iced::mouse::Button::Middle,
            )),
            iced::Event::Mouse(iced::mouse::Event::ButtonReleased(
                iced::mouse::Button::Middle,
            )),
        ]);

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Close]);
    }

    #[test]
    fn given_row_with_folder_when_rendered_then_title_and_folder_show() {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(
            container(
                SessionRow::new("zsh", &palette)
                    .folder("otty")
                    .on_press(Message::Open),
            )
            .width(260),
        );

        ui.find("zsh").expect("title should be visible");
        ui.click("otty").expect("folder should be visible");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Open]);
    }

    #[test]
    fn given_each_tone_when_color_read_then_it_is_its_token() {
        let palette = UiPalette::otty_dark();

        assert_eq!(SessionTone::Muted.color(&palette), palette.text_muted());
        assert_eq!(SessionTone::Waiting.color(&palette), palette.status_warn());
        assert_eq!(SessionTone::Done.color(&palette), palette.status_ok());
        assert_eq!(SessionTone::Failed.color(&palette), palette.status_error());
    }
}

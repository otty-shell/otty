//! `SessionRow`: one session under a group label.

use iced::widget::{button, container, hover, mouse_area, row};
use iced::{Alignment, Border, Color, Element, Length, Padding};

use super::ellipsis_text::EllipsisText;
use super::glyph;
use super::icon_button::{IconButton, IconButtonSize};
use crate::palette::UiPalette;
use crate::{fonts, radius, size, spacing, typography};

/// Gap between title, state and close (`gap: 8px`).
const GAP: f32 = spacing::SPACE_3;
/// Right padding of the row (`padding: 0 8px`).
const PADDING_RIGHT: f32 = spacing::SPACE_3;

/// Color of the state word of a [`SessionRow`], and of the count and state
/// word of a [`TreeRow`] (`.ot-state-*`).
///
/// [`TreeRow`]: super::tree_row::TreeRow
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
/// Everything stays on one line. The title takes the room first, then the
/// state word; the muted folder takes what is left. A text that does not fit
/// ends with "…", or disappears when there is no room at all. The close
/// cross is set with `on_close` and shows only while the pointer is over the
/// row. The row always keeps room for it, so nothing shifts when it appears
/// and no text reaches it.
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

        // The title takes the room it needs first, then the state word. The
        // folder in between takes what is left and ends with "…" when that is
        // too little. Row layout places the fixed-width items before the
        // filling one, whatever their order on screen.
        let title_color = if is_selected {
            palette.text_strong()
        } else {
            palette.text_body()
        };
        let mut content = row![
            EllipsisText::new(value.title, body.size(), fonts::UI, title_color)
                .shrink(0.0),
        ]
        .spacing(GAP)
        .height(Length::Fill)
        .align_y(Alignment::Center);
        if let Some(folder) = value.folder {
            content = content.push(EllipsisText::new(
                folder,
                caption.size(),
                fonts::MONO,
                palette.text_muted(),
            ));
        }
        if let Some((word, tone)) = value.state {
            content = content.push(
                EllipsisText::new(
                    word,
                    caption.size(),
                    fonts::UI,
                    tone.color(&palette),
                )
                .shrink(0.0),
            );
        }
        let has_close = value.on_close.is_some();

        // The cross is drawn over the room kept at the right edge, so the
        // text never reaches it and nothing shifts when it appears.
        let row_palette = palette.clone();
        let control = button(content)
            .width(Length::Fill)
            .height(size::ROW)
            .padding(Padding {
                top: 0.0,
                right: if has_close {
                    size::CONTROL_SM + GAP
                } else {
                    PADDING_RIGHT
                },
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
    use crate::test_support::TempDir;

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

    #[test]
    fn given_selected_row_without_hover_then_close_cross_is_not_drawn() {
        let folder = TempDir::new("session-row");
        let reference = folder.path().join("hovered");

        assert!(drawn_row_matches(true, true, &reference));
        assert!(!drawn_row_matches(true, false, &reference));
        assert!(drawn_row_matches(true, true, &reference));
    }

    #[test]
    fn given_row_when_pointer_over_it_then_close_cross_is_drawn() {
        let folder = TempDir::new("session-row");
        let reference = folder.path().join("with-close");

        assert!(drawn_row_matches(true, true, &reference));
        assert!(!drawn_row_matches(false, true, &reference));
    }

    #[test]
    fn given_close_label_when_cross_hovered_then_the_tooltip_shows_that_label()
    {
        let folder = TempDir::new("session-row");
        let reference = folder.path().join("close-label");

        assert!(hovered_cross_matches("Close session", &reference));
        assert!(!hovered_cross_matches("Fermer la session", &reference));
        assert!(hovered_cross_matches("Close session", &reference));
    }

    #[test]
    fn given_row_without_close_then_hover_draws_no_cross() {
        let folder = TempDir::new("session-row");
        let reference = folder.path().join("no-close");

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

    /// Bounds of the title, the folder and the state of a row with a close
    /// cross in a space `width` wide.
    fn row_bounds(
        width: f32,
    ) -> (iced::Rectangle, iced::Rectangle, iced::Rectangle) {
        let palette = UiPalette::otty_dark();
        let row: Element<'_, Message> = SessionRow::new("claude", &palette)
            .folder("otty-project")
            .state("needs your input", SessionTone::Waiting)
            .on_press(Message::Open)
            .on_close(Message::Close, "Close session")
            .into();
        let mut ui = simulator(container(row).width(width));
        let title = ui.find("claude").expect("title").bounds();
        let folder = ui.find("otty-project").expect("folder").bounds();
        let state = ui.find("needs your input").expect("state").bounds();

        (title, folder, state)
    }

    #[test]
    fn given_narrow_row_when_laid_out_then_the_title_keeps_its_width_and_notes_give_way()
     {
        let (full_title, full_folder, full_state) = row_bounds(600.0);

        let (title, folder, state) = row_bounds(170.0);

        assert!(
            (title.width - full_title.width).abs() < 0.5,
            "the title should keep its width: {title:?} against {full_title:?}"
        );
        assert!(
            folder.width < full_folder.width || state.width < full_state.width,
            "a note should shrink: {folder:?} {state:?}"
        );
        for note in [folder, state] {
            assert!(note.height <= full_folder.height + 0.5, "one line");
            assert!(
                note.x + note.width <= 170.0 - size::CONTROL_SM + 0.5,
                "{note:?} must stay clear of the close cross"
            );
        }
    }

    #[test]
    fn given_minimum_panel_width_when_laid_out_then_text_stays_on_one_line_clear_of_close()
     {
        let (full_title, _, _) = row_bounds(600.0);

        let (title, folder, state) = row_bounds(128.0);

        assert!(
            (title.height - full_title.height).abs() < 0.5,
            "the title should stay one line: {title:?}"
        );
        assert!(
            (title.width - full_title.width).abs() < 0.5,
            "the title should take the room first: {title:?}"
        );
        for text in [title, folder, state] {
            assert!(
                text.x + text.width <= 128.0 - size::CONTROL_SM + 0.5,
                "{text:?} must stay clear of the close cross"
            );
        }
        assert!(
            title.x + title.width <= folder.x + 0.5 || folder.width < 1.0,
            "the folder must not overlap the title"
        );
    }

    #[test]
    fn given_very_narrow_row_when_laid_out_then_the_title_ends_with_an_ellipsis()
     {
        let (full_title, _, _) = row_bounds(600.0);

        let (title, _, _) = row_bounds(70.0);

        assert!(
            title.width < full_title.width,
            "the title should be cut: {title:?} against {full_title:?}"
        );
        assert!(
            title.x + title.width <= 70.0 - size::CONTROL_SM + 0.5,
            "the title must stay clear of the close cross"
        );
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

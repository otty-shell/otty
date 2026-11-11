//! `FormPage`: a page that holds a form.

use iced::widget::{Space, column, container, row};
use iced::{Alignment, Element, Length, Padding, alignment};

use super::divider;
use crate::palette::UiPalette;
use crate::{size, spacing};

/// Smallest height of the footer bar, its line included (`.form-foot`:
/// `height: 60px`). It grows with a footer that needs more, such as a hint on
/// several lines.
const FOOTER_BAR_HEIGHT: f32 = 60.0;

/// One page that holds a form: sections `space-6` apart and an optional footer
/// with the form's buttons. It only lays them out: on screen it goes in a
/// [`ScreenArea`], whose one scroller moves the fields and the buttons together.
///
/// The page title is the caller's first section. The sections are at most
/// `size::FORM` wide, centered horizontally, and have `space-7` padding. The
/// footer's content sits in the same column, so its buttons line up with the
/// right edge of the form.
///
/// [`ScreenArea`]: super::screen_area::ScreenArea
///
/// # Example
///
/// ```
/// use iced::Element;
/// use iced::widget::text;
/// use otty_ui_kit::components::form_page::FormPage;
/// use otty_ui_kit::palette::UiPalette;
///
/// let palette = UiPalette::otty_dark();
/// let page: Element<'_, ()> = FormPage::new(&palette)
///     .section(text("Settings"))
///     .section(text("General"))
///     .footer(text("Save"))
///     .into();
/// ```
pub struct FormPage<'a, Message> {
    palette: UiPalette,
    sections: Vec<Element<'a, Message>>,
    footer: Option<Element<'a, Message>>,
}

impl<'a, Message> FormPage<'a, Message> {
    /// Creates an empty page.
    pub fn new(palette: &UiPalette) -> Self {
        Self {
            palette: palette.clone(),
            sections: Vec::new(),
            footer: None,
        }
    }

    /// Appends a section.
    pub fn section(mut self, section: impl Into<Element<'a, Message>>) -> Self {
        self.sections.push(section.into());
        self
    }

    /// Sets the footer: the form's buttons, and a hint on their left if there
    /// is one. It sits in a bar as wide as the page (`.form-foot`: 60 high or
    /// more, a line on top), in the same column as the sections and aligned to
    /// its right. While the form is lower than the room the bar is at the
    /// bottom of it; otherwise it comes after the last section.
    pub fn footer(mut self, footer: impl Into<Element<'a, Message>>) -> Self {
        self.footer = Some(footer.into());
        self
    }
}

impl<'a, Message: 'a> From<FormPage<'a, Message>> for Element<'a, Message> {
    fn from(value: FormPage<'a, Message>) -> Self {
        let mut sections = column![]
            .spacing(spacing::SPACE_6)
            .width(Length::Fill)
            .max_width(size::FORM);
        for section in value.sections {
            sections = sections.push(section);
        }

        let body = container(sections)
            .padding(spacing::SPACE_7)
            .center_x(Length::Fill)
            .height(Length::Fill);
        let mut page = column![body].width(Length::Fill).height(Length::Fill);
        if let Some(footer) = value.footer {
            page = page
                .push(divider::horizontal(&value.palette))
                .push(footer_bar(footer));
        }

        page.into()
    }
}

/// The bar under the sections, without its line: the column of the sections,
/// with the same side padding. The empty space holds the bar open to its
/// smallest height, and the footer is centered beside it.
fn footer_bar<'a, Message: 'a>(
    footer: Element<'a, Message>,
) -> Element<'a, Message> {
    let padding = spacing::SPACE_3;
    let open =
        Space::new().height(FOOTER_BAR_HEIGHT - size::LINE - 2.0 * padding);
    let footer = container(footer)
        .width(Length::Fill)
        .align_x(alignment::Horizontal::Right);
    let form_column = container(row![open, footer].align_y(Alignment::Center))
        .width(Length::Fill)
        .max_width(size::FORM);

    container(form_column)
        .center_x(Length::Fill)
        .padding(Padding::from([padding, spacing::SPACE_7]))
        .into()
}

#[cfg(test)]
mod tests {
    use iced::widget::{button, column, row, text, text_input};
    use iced::{Event, Point, Rectangle, mouse};
    use iced_test::selector::Candidate;
    use iced_test::{Simulator, simulator};

    use super::*;
    use crate::components::screen_area::ScreenArea;
    use crate::test_support::TempDir;

    /// The smallest width of a form's content: a screen's, without the side
    /// padding of the page.
    const MIN_WIDTH: f32 = size::SCREEN_MIN - 2.0 * spacing::SPACE_7;

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Saved,
        Typed(String),
        Ran,
    }

    /// A page whose first line ends with Save, with an input under it, `rows`
    /// more lines and the word "end" in the far corner.
    fn form(rows: usize) -> FormPage<'static, Message> {
        let palette = UiPalette::otty_dark();
        let mut page = FormPage::new(&palette)
            .section(text("start").width(Length::Fill))
            .section(row![
                iced::widget::Space::new().width(Length::Fill),
                button(text("Save")).on_press(Message::Saved),
            ])
            .section(
                text_input("", "")
                    .id(iced::widget::Id::new("name"))
                    .on_input(Message::Typed),
            );
        for index in 0..rows {
            page = page.section(text(format!("line {index}")));
        }
        page.section(row![
            iced::widget::Space::new().width(Length::Fill),
            text("end"),
        ])
    }

    /// The page in the area it has in the app.
    fn in_window<'a>(
        page: FormPage<'a, Message>,
        width: f32,
        height: f32,
    ) -> Simulator<'a, Message> {
        Simulator::with_size(
            iced::Settings::default(),
            (width, height),
            ScreenArea::new(0, page, &UiPalette::otty_dark()),
        )
    }

    /// Scrolls under `pointer` as far as `x` and `y` go. As in the tables, a
    /// negative number scrolls right or down.
    fn wheel(ui: &mut Simulator<'_, Message>, pointer: Point, x: f32, y: f32) {
        ui.point_at(pointer);
        let _ = ui.simulate([Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Pixels { x, y },
        })]);
    }

    /// Top left corner of `label` on the screen. It must be in view, since
    /// the visible part is all a test can read.
    fn place(ui: &mut Simulator<'_, Message>, label: &str) -> Point {
        let bounds = ui
            .find(label)
            .expect("text should exist")
            .visible_bounds()
            .expect("text should be in view");

        Point::new(bounds.x, bounds.y)
    }

    #[test]
    fn given_less_room_than_the_minimum_when_laid_out_then_the_form_keeps_it() {
        let mut ui = in_window(form(0), 300.0, 500.0);

        let start = ui.find("start").expect("marker should exist").bounds();

        assert_eq!(start.width, MIN_WIDTH);
        assert_eq!(start.x, spacing::SPACE_7);
    }

    #[test]
    fn given_more_room_than_the_minimum_when_laid_out_then_the_form_fills_it_and_does_not_scroll()
     {
        let width = size::SCREEN_MIN + 8.0;
        let mut ui = in_window(form(0), width, 500.0);

        let start = ui.find("start").expect("marker should exist").bounds();
        wheel(&mut ui, Point::new(100.0, 100.0), -10_000.0, 0.0);
        let after = place(&mut ui, "start");

        assert_eq!(start.width, width - 2.0 * spacing::SPACE_7);
        assert_eq!(after, Point::new(start.x, start.y));
    }

    #[test]
    fn given_less_room_than_the_minimum_when_scrolled_sideways_then_a_far_control_can_be_pressed()
     {
        let mut ui = in_window(form(0), 300.0, 500.0);
        let pressed_before = ui.click("Save").is_ok();
        let before: Vec<Message> = ui.into_messages().collect();
        let mut ui = in_window(form(0), 300.0, 500.0);

        wheel(&mut ui, Point::new(100.0, 100.0), -10_000.0, 0.0);
        ui.click("Save").expect("Save should be in reach");
        let after: Vec<Message> = ui.into_messages().collect();

        assert!(!pressed_before || before.is_empty());
        assert_eq!(after, vec![Message::Saved]);
    }

    #[test]
    fn given_a_scrolled_narrow_page_when_an_input_is_pressed_then_typing_reaches_it()
     {
        let mut ui = in_window(form(0), 300.0, 500.0);

        wheel(&mut ui, Point::new(100.0, 100.0), -60.0, 0.0);
        ui.click(iced::widget::Id::new("name"))
            .expect("the input should be in reach");
        let _ = ui.typewrite("n");

        let typed: Vec<Message> = ui.into_messages().collect();
        assert_eq!(typed, vec![Message::Typed(String::from("n"))]);
    }

    #[test]
    fn given_a_focused_input_when_scrolled_out_of_view_then_typing_still_reaches_it()
     {
        let mut ui = in_window(form(40), 300.0, 300.0);

        ui.click(iced::widget::Id::new("name"))
            .expect("the input should be in reach");
        wheel(&mut ui, Point::new(100.0, 100.0), -10_000.0, -10_000.0);
        let _ = ui.typewrite("n");

        let typed: Vec<Message> = ui.into_messages().collect();
        assert_eq!(typed, vec![Message::Typed(String::from("n"))]);
    }

    #[test]
    fn given_a_page_that_overflows_when_its_bars_show_then_the_form_does_not_move()
     {
        let mut fits = in_window(form(0), 300.0, 900.0);
        let mut overflows = in_window(form(40), 300.0, 300.0);

        let fits = fits.find("start").expect("marker should exist").bounds();
        let overflows = overflows
            .find("start")
            .expect("marker should exist")
            .bounds();

        assert_eq!(fits, overflows);
    }

    /// A page with a footer: `hint` on the left and Run on the right.
    fn page_with_footer(rows: usize) -> FormPage<'static, Message> {
        form(rows).footer(row![
            text("hint").width(Length::Fill),
            button(text("Run")).on_press(Message::Ran),
        ])
    }

    /// Where the scrollers of the page are.
    fn scrollers(ui: &mut Simulator<'_, Message>) -> Vec<Rectangle> {
        let mut found = Vec::new();

        let _ = ui.find(|candidate: Candidate<'_>| {
            if let Candidate::Scrollable { bounds, .. } = candidate {
                found.push(bounds);
            }
            None::<()>
        });

        found
    }

    #[test]
    fn given_a_footer_when_laid_out_then_the_whole_page_is_one_scroller() {
        let (width, height) = (300.0, 300.0);
        let mut ui = in_window(page_with_footer(40), width, height);

        let found = scrollers(&mut ui);

        assert_eq!(
            found,
            vec![Rectangle::new(
                Point::ORIGIN,
                iced::Size::new(width, height)
            )]
        );
    }

    #[test]
    fn given_a_form_shorter_than_the_room_when_laid_out_then_the_footer_is_at_the_bottom()
     {
        let height = 768.0;
        let mut ui = in_window(page_with_footer(0), 1024.0, height);

        let run = place(&mut ui, "Run");
        let hint = place(&mut ui, "hint");
        let found = ui.find("Run").expect("Run should exist").bounds();

        let middle = height - (FOOTER_BAR_HEIGHT - size::LINE) / 2.0;
        assert!(
            (run.y + found.height / 2.0 - middle).abs() < 1.0,
            "Run at {run:?} should be in the middle of the bar at {middle}"
        );
        assert!(hint.y > height - FOOTER_BAR_HEIGHT);
    }

    #[test]
    fn given_a_form_shorter_than_the_room_when_scrolled_then_nothing_moves() {
        let mut ui = in_window(page_with_footer(0), 1024.0, 768.0);
        let before = place(&mut ui, "Run");

        wheel(&mut ui, Point::new(100.0, 100.0), -10_000.0, -10_000.0);

        assert_eq!(place(&mut ui, "Run"), before);
    }

    #[test]
    fn given_a_form_taller_than_the_room_when_laid_out_then_the_footer_comes_after_the_fields_and_scrolls_with_them()
     {
        let height = 300.0;
        let mut ui = in_window(page_with_footer(40), 1024.0, height);

        let last = ui.find("end").expect("last section").bounds();
        let hidden = ui.find("Run").expect("Run should exist");
        let run = hidden.bounds();
        wheel(&mut ui, Point::new(100.0, 100.0), 0.0, -10_000.0);
        ui.click("Run").expect("Run should be in reach");

        let pressed: Vec<Message> = ui.into_messages().collect();
        assert_eq!(hidden.visible_bounds(), None);
        assert!(run.y > last.y + last.height, "{run:?} under {last:?}");
        assert_eq!(pressed, vec![Message::Ran]);
    }

    #[test]
    fn given_less_room_than_the_minimum_when_scrolled_sideways_then_the_fields_and_the_footer_move_together()
     {
        let mut ui = in_window(page_with_footer(0), 300.0, 500.0);
        let start = place(&mut ui, "start");
        let hint = place(&mut ui, "hint");

        wheel(&mut ui, Point::new(100.0, 100.0), -20.0, 0.0);

        assert_eq!(hint.x, spacing::SPACE_7);
        assert_eq!(
            place(&mut ui, "start"),
            Point::new(start.x - 20.0, start.y)
        );
        assert_eq!(place(&mut ui, "hint"), Point::new(hint.x - 20.0, hint.y));
    }

    #[test]
    fn given_less_room_than_the_minimum_when_laid_out_then_the_footer_keeps_the_width_of_the_form()
     {
        let mut ui = in_window(page_with_footer(0), 300.0, 500.0);

        let hint = ui.find("hint").expect("hint should exist").bounds();
        let run = ui.find("Run").expect("Run should exist").bounds();

        let form_right = spacing::SPACE_7 + MIN_WIDTH;
        assert_eq!(hint.x, spacing::SPACE_7);
        assert!(run.x + run.width <= form_right);
        assert!(run.x + run.width > form_right - 20.0);
    }

    /// A page with a footer of two texts, one at each of its ends.
    fn page_with_footer_edges() -> FormPage<'static, Message> {
        form(0).footer(row![text("hint").width(Length::Fill), text("edge")])
    }

    /// The left edge of the fields and of the footer, then the right edge of
    /// each. The fields are the lines "start" and "end" of [`form`].
    fn edges(ui: &mut Simulator<'_, Message>) -> (f32, f32, f32, f32) {
        let start = ui.find("start").expect("start").bounds();
        let end = ui.find("end").expect("end").bounds();
        let hint = ui.find("hint").expect("hint").bounds();
        let edge = ui.find("edge").expect("edge").bounds();

        (start.x, hint.x, end.x + end.width, edge.x + edge.width)
    }

    fn assert_near(found: f32, expected: f32, what: &str) {
        assert!(
            (found - expected).abs() < 0.01,
            "{what}: {found} should be {expected}"
        );
    }

    /// The odd widths put the column on half a pixel, and the first one is
    /// just past the width where the form stops filling the page.
    #[test]
    fn given_a_window_wider_than_the_form_when_laid_out_then_the_footer_lines_up_with_it()
     {
        let fills = size::FORM + 2.0 * spacing::SPACE_7;

        for width in [fills + 1.0, 1024.0, 1400.0, 1401.0] {
            let mut ui = in_window(page_with_footer_edges(), width, 500.0);

            let (start, hint, end, edge) = edges(&mut ui);

            assert_near(start, (width - size::FORM) / 2.0, "fields");
            assert_near(hint, start, "the left of the footer");
            assert_near(edge, end, "the right of the footer");
            assert_near(edge, (width + size::FORM) / 2.0, "the form's edge");
        }
    }

    /// Up to this width the form fills the page, so the footer is where it
    /// always was.
    #[test]
    fn given_a_window_where_the_form_fills_the_page_when_laid_out_then_the_footer_spans_it()
     {
        let fills = size::FORM + 2.0 * spacing::SPACE_7;

        for width in [size::SCREEN_MIN, 700.0, fills] {
            let mut ui = in_window(page_with_footer_edges(), width, 500.0);

            let (start, hint, end, edge) = edges(&mut ui);

            assert_near(start, spacing::SPACE_7, "fields");
            assert_near(hint, start, "the left of the footer");
            assert_near(end, width - spacing::SPACE_7, "the fields' edge");
            assert_near(edge, end, "the right of the footer");
        }
    }

    #[test]
    fn given_a_footer_that_only_holds_buttons_when_laid_out_then_they_sit_at_the_right_of_the_form()
     {
        let width = 1024.0;
        let page = form(0).footer(button(text("Run")).on_press(Message::Ran));
        let mut ui = in_window(page, width, 500.0);

        let run = ui.find("Run").expect("Run should exist").bounds();

        let form_right = (width + size::FORM) / 2.0;
        assert!(run.x + run.width <= form_right);
        assert!(run.x + run.width > form_right - 20.0);
    }

    /// The line is the width of the page, not of the column. The bar is drawn
    /// with the same footer in a window where the column fills the page and in
    /// a wide one, and there the line must go on past the column.
    #[test]
    fn given_a_wide_window_when_drawn_then_the_line_spans_the_page() {
        let folder = TempDir::new("form-page-line");
        let reference = folder.path().join("line");
        let palette = UiPalette::otty_dark();
        let (width, height) = (1400.0, 200.0);
        let line_only: Element<'_, Message> = column![
            Space::new().height(Length::Fill),
            divider::horizontal(&palette),
            Space::new().height(FOOTER_BAR_HEIGHT - size::LINE),
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .into();
        let page: FormPage<'_, Message> =
            FormPage::new(&palette).footer(Space::new());

        let line_matches = Simulator::with_size(
            iced::Settings::default(),
            (width, height),
            line_only,
        )
        .snapshot(&iced::Theme::Dark)
        .expect("the line should draw")
        .matches_hash(&reference)
        .expect("reference should be readable");
        let page_matches = in_window(page, width, height)
            .snapshot(&iced::Theme::Dark)
            .expect("the page should draw")
            .matches_hash(&reference)
            .expect("reference should be readable");

        assert!(line_matches);
        assert!(page_matches);
    }

    #[test]
    fn given_a_footer_higher_than_the_bar_when_laid_out_then_the_bar_grows() {
        let height = 768.0;
        let page = form(0).footer(column![
            text("first"),
            iced::widget::Space::new().height(2.0 * FOOTER_BAR_HEIGHT),
            text("last"),
        ]);
        let mut ui = in_window(page, 1024.0, height);

        let first = place(&mut ui, "first");
        let last = ui.find("last").expect("last should exist");
        let last = last.visible_bounds().expect("last should be in view");

        assert!(first.y < height - 2.0 * FOOTER_BAR_HEIGHT);
        assert!(last.y + last.height <= height);
    }

    #[test]
    fn given_a_form_without_a_footer_when_laid_out_then_there_is_no_line_or_bar_and_it_starts_at_the_top()
     {
        let mut ui = in_window(form(0), 1024.0, 768.0);

        let start = ui.find("start").expect("marker should exist").bounds();

        assert_eq!(start.y, spacing::SPACE_7);
    }

    #[test]
    fn given_wide_window_when_laid_out_then_form_is_centered() {
        let palette = UiPalette::otty_dark();
        let page: Element<'_, ()> = FormPage::new(&palette)
            .section(container(text("start")).width(Length::Fill))
            .into();
        let mut ui = simulator(page);

        let start = ui.find("start").expect("marker should be visible");

        let window_width = 1024.0;
        let expected_x = (window_width - size::FORM) / 2.0;
        assert!(
            (start.bounds().x - expected_x).abs() < 0.5,
            "form left edge {} should be {expected_x}",
            start.bounds().x
        );
    }
}

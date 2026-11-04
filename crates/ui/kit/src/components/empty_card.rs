//! `EmptyCard`: centered start card shown when no session is open.

use iced::widget::{column, container, text};
use iced::{Element, Length};

use crate::palette::UiPalette;
use crate::{fonts, size, spacing, typography};

/// Column with the logo, a title, a muted description and a stack of
/// actions such as [`ActionTile`]s. It is 460 px wide. It only draws itself: on
/// screen it goes in a [`ScreenArea`], which centers it and scrolls.
///
/// The caller passes the logo as an element and the card never recolors it.
/// The card gives it a slot of `EMPTY_LOGO_WIDTH` by `EMPTY_LOGO_HEIGHT`,
/// so give the logo `Length::Fill` on both sides.
///
/// [`ActionTile`]: super::action_tile::ActionTile
/// [`ScreenArea`]: super::screen_area::ScreenArea
///
/// # Example
///
/// ```
/// use iced::Element;
/// use iced::widget::text;
/// use otty_ui_kit::components::action_tile::ActionTile;
/// use otty_ui_kit::components::empty_card::EmptyCard;
/// use otty_ui_kit::palette::UiPalette;
///
/// let palette = UiPalette::otty_dark();
/// let icon = iced::widget::svg::Handle::from_memory(&b"<svg/>"[..]);
/// let card: Element<'_, ()> = EmptyCard::new(
///     "Start a session",
///     "Open a terminal on this computer.",
///     text("logo"),
///     &palette,
/// )
/// .action(ActionTile::new(icon, "New terminal", &palette))
/// .into();
/// ```
pub struct EmptyCard<'a, Message> {
    title: String,
    description: String,
    logo: Element<'a, Message>,
    palette: UiPalette,
    actions: Vec<Element<'a, Message>>,
}

impl<'a, Message> EmptyCard<'a, Message> {
    /// Creates a card with a `title`, a `description` and the `logo`.
    pub fn new(
        title: impl Into<String>,
        description: impl Into<String>,
        logo: impl Into<Element<'a, Message>>,
        palette: &UiPalette,
    ) -> Self {
        Self {
            title: title.into(),
            description: description.into(),
            logo: logo.into(),
            palette: palette.clone(),
            actions: Vec::new(),
        }
    }

    /// Adds an action under the description.
    pub fn action(mut self, action: impl Into<Element<'a, Message>>) -> Self {
        self.actions.push(action.into());
        self
    }
}

impl<'a, Message: 'a> From<EmptyCard<'a, Message>> for Element<'a, Message> {
    fn from(value: EmptyCard<'a, Message>) -> Self {
        let palette = value.palette;
        let title_style = typography::TITLE;
        let body = typography::BODY;

        let logo = container(value.logo)
            .width(size::EMPTY_LOGO_WIDTH)
            .height(size::EMPTY_LOGO_HEIGHT);
        let heading = column![
            text(value.title)
                .size(title_style.size())
                .line_height(iced::Pixels(title_style.line_height()))
                .font(fonts::UI_SEMIBOLD)
                .color(palette.text_strong()),
            text(value.description)
                .size(body.size())
                .line_height(iced::Pixels(body.line_height()))
                .font(fonts::UI)
                .color(palette.text_muted()),
        ]
        .spacing(spacing::SPACE_1);
        let actions = column(value.actions).spacing(spacing::SPACE_2);

        container(column![logo, heading, actions].spacing(spacing::SPACE_5))
            .width(Length::Fixed(size::EMPTY_CARD))
            .into()
    }
}

#[cfg(test)]
mod tests {
    use iced::widget::{button, row, text};
    use iced::{Event, Point, Size, Theme, mouse};
    use iced_test::selector::Candidate;
    use iced_test::{Simulator, simulator};

    use super::*;
    use crate::components::action_tile::ActionTile;
    use crate::components::glyph;
    use crate::components::screen_area::ScreenArea;
    use crate::test_support::TempDir;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Message {
        NewTerminal,
        NewLaunch,
    }

    fn card() -> EmptyCard<'static, Message> {
        let palette = UiPalette::otty_dark();

        EmptyCard::new(
            "Start a session",
            "Open a terminal on this computer.",
            text("logo"),
            &palette,
        )
        .action(
            ActionTile::new(glyph::plus(), "New terminal", &palette)
                .on_press(Message::NewTerminal),
        )
        .action(
            ActionTile::new(glyph::plus(), "New launch…", &palette)
                .on_press(Message::NewLaunch),
        )
    }

    #[test]
    fn given_empty_card_then_title_and_description_are_shown() {
        let mut ui = simulator(card());

        assert!(ui.find("Start a session").is_ok());
        assert!(ui.find("Open a terminal on this computer.").is_ok());
        assert!(ui.find("logo").is_ok());
    }

    #[test]
    fn given_empty_card_when_second_tile_pressed_then_only_its_message_is_sent()
    {
        let mut ui = simulator(card());

        ui.click("New launch…").expect("tile should be visible");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::NewLaunch]);
    }

    /// A window where a card of [`tall_card`] fits both ways.
    const ROOMY: Size = Size::new(1024.0, 768.0);
    /// Narrower than the card, and so than the minimum of a screen.
    const NARROW: f32 = 300.0;
    /// Lower than a card of [`tall_card`].
    const SHORT: f32 = 200.0;
    /// Far enough to reach the end of any page here.
    const FAR: f32 = -10_000.0;

    /// A card with `rows` lines, as wide as the card, and a last action at its
    /// right end.
    fn tall_card(rows: usize) -> EmptyCard<'static, Message> {
        let palette = UiPalette::otty_dark();
        let mut card = EmptyCard::new("Title", "Text", text("logo"), &palette)
            .action(text("whole width").width(Length::Fill));

        for index in 0..rows {
            card = card.action(text(format!("line {index}")));
        }

        card.action(row![
            iced::widget::Space::new().width(Length::Fill),
            button(text("Last")).on_press(Message::NewLaunch),
        ])
    }

    /// The card in the area it has in the app.
    fn page_in_window(
        rows: usize,
        window: impl Into<Size>,
    ) -> Simulator<'static, Message> {
        Simulator::with_size(
            iced::Settings::default(),
            window,
            ScreenArea::new(0, tall_card(rows), &UiPalette::otty_dark()),
        )
    }

    /// Scrolls under the middle of the window. As in the tables, a negative
    /// number scrolls right or down.
    fn wheel(ui: &mut Simulator<'_, Message>, x: f32, y: f32) {
        ui.point_at(Point::new(150.0, 100.0));
        let _ = ui.simulate([Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Pixels { x, y },
        })]);
    }

    /// How far the page is scrolled, how large its content is and how large
    /// the part that shows is.
    fn page_scroller(ui: &mut Simulator<'_, Message>) -> (Point, Size, Size) {
        ui.find(|candidate: Candidate<'_>| match candidate {
            Candidate::Scrollable {
                bounds,
                content_bounds,
                translation,
                ..
            } => Some((
                Point::new(translation.x, translation.y),
                content_bounds.size(),
                bounds.size(),
            )),
            _ => None,
        })
        .expect("the page should scroll")
    }

    #[test]
    fn given_room_for_the_card_when_laid_out_then_it_is_centered_and_nothing_scrolls()
     {
        let mut ui = page_in_window(0, ROOMY);

        let logo = ui.find("logo").expect("the logo should show").bounds();
        let last = ui.find("Last").expect("the action should show").bounds();
        let whole = ui.find("whole width").expect("line").bounds();
        let (offset, content, showing) = page_scroller(&mut ui);
        wheel(&mut ui, FAR, FAR);
        let (after_wheel, _, _) = page_scroller(&mut ui);

        assert_eq!(whole.width, size::EMPTY_CARD);
        assert_eq!(whole.x, (ROOMY.width - size::EMPTY_CARD) / 2.0);
        assert!(
            logo.y > 100.0 && logo.y < ROOMY.height - last.y,
            "the card should sit in the middle: {logo:?} and {last:?}"
        );
        assert_eq!(content, showing, "no bar should show");
        assert_eq!(offset, Point::ORIGIN);
        assert_eq!(after_wheel, Point::ORIGIN);
    }

    /// The page must draw what a card in a centered container draws, with no
    /// bar, at any size where the card fits.
    #[test]
    fn given_the_card_fits_when_drawn_then_it_looks_like_a_card_in_a_centered_container()
     {
        let folder = TempDir::new("empty-card");

        for (index, window) in [
            ROOMY,
            Size::new(701.0, 501.0),
            Size::new(size::SCREEN_MIN, 500.0),
        ]
        .into_iter()
        .enumerate()
        {
            let reference = folder.path().join(format!("window-{index}"));
            let plain: Element<'_, Message> = container(tall_card(2))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into();

            let plain_matches =
                Simulator::with_size(iced::Settings::default(), window, plain)
                    .snapshot(&Theme::Dark)
                    .expect("the card should draw")
                    .matches_hash(&reference)
                    .expect("reference should be readable");
            let page_matches = page_in_window(2, window)
                .snapshot(&Theme::Dark)
                .expect("the page should draw")
                .matches_hash(&reference)
                .expect("reference should be readable");

            assert!(plain_matches);
            assert!(page_matches, "at {window:?}");
        }
    }

    /// The minimum of a screen is wider than the card, so the card never gets
    /// narrower. It stays in the middle of that minimum and the area scrolls.
    #[test]
    fn given_less_room_than_the_minimum_then_the_card_keeps_its_width_and_a_far_action_can_be_pressed()
     {
        let mut ui = page_in_window(0, (NARROW, 600.0));

        let whole = ui.find("whole width").expect("line").bounds();
        wheel(&mut ui, FAR, 0.0);
        let (offset, content, showing) = page_scroller(&mut ui);
        ui.click("Last").expect("the action should be in reach");

        let sent: Vec<Message> = ui.into_messages().collect();
        assert_eq!(whole.width, size::EMPTY_CARD);
        assert_eq!(whole.x, (size::SCREEN_MIN - size::EMPTY_CARD) / 2.0);
        assert_eq!(content.height, showing.height, "no bar down");
        assert_eq!(offset.x, size::SCREEN_MIN - NARROW);
        assert_eq!(offset.y, 0.0);
        assert_eq!(sent, vec![Message::NewLaunch]);
    }

    #[test]
    fn given_a_room_lower_than_the_card_then_it_starts_at_the_top_and_scrolls_to_its_last_action()
     {
        let mut ui = page_in_window(10, (size::SCREEN_MIN, SHORT));

        let logo = ui.find("logo").expect("the logo should show").bounds();
        let (_, content, showing) = page_scroller(&mut ui);
        wheel(&mut ui, 0.0, FAR);
        let (offset, _, _) = page_scroller(&mut ui);
        ui.click("Last").expect("the action should be in reach");

        let sent: Vec<Message> = ui.into_messages().collect();
        assert_eq!(logo.y, 0.0);
        assert!(content.height > showing.height);
        assert_eq!(content.width, showing.width, "no bar sideways");
        assert!((offset.y - (content.height - showing.height)).abs() < 1.0);
        assert_eq!(sent, vec![Message::NewLaunch]);
    }

    #[test]
    fn given_a_card_too_tall_when_its_bar_shows_then_the_width_of_the_card_does_not_change()
     {
        let mut fits = page_in_window(10, (NARROW, 900.0));
        let mut overflows = page_in_window(10, (NARROW, SHORT));

        let fits = fits.find("whole width").expect("line").bounds();
        let overflows = overflows.find("whole width").expect("line").bounds();

        assert_eq!(fits.width, overflows.width);
        assert_eq!(fits.x, overflows.x);
    }
}

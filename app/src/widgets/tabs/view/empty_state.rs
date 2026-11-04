//! Empty state shown in the main area when no session is open.

use std::sync::OnceLock;

use iced::widget::{image, svg};
use iced::{Element, Length};
use otty_ui_kit::components::action_tile::ActionTile;
use otty_ui_kit::components::empty_card::EmptyCard;

use crate::i18n::{self, Key};
use crate::icons;
use crate::shortcuts::AppShortcut;
use crate::theme::ThemeProps;

/// Props for the empty state.
pub(crate) struct EmptyStateProps<'a, Message> {
    pub(crate) theme: ThemeProps<'a>,
    /// Message sent by the New terminal tile.
    pub(crate) on_new_terminal: Message,
    /// Message sent by the New launch tile.
    pub(crate) on_new_launch: Message,
}

/// Render the start card: the logo, a title, a description and two tiles, New
/// terminal and New launch.
pub(crate) fn view<'a, Message: Clone + 'a>(
    props: EmptyStateProps<'a, Message>,
) -> Element<'a, Message> {
    let palette = props.theme.theme.ui();

    // The logo keeps its own colors; the card only gives it a slot.
    let logo = image(logo_handle().clone())
        .width(Length::Fill)
        .height(Length::Fill);

    let new_terminal = ActionTile::new(
        svg::Handle::from_memory(icons::ADD_TAB_HEADER),
        i18n::t(Key::EmptyNewTerminal),
        palette,
    )
    .shortcut(AppShortcut::NewTerminal.label())
    .accent(true)
    .on_press(props.on_new_terminal);

    let new_launch = ActionTile::new(
        svg::Handle::from_memory(icons::PLAY),
        i18n::t(Key::EmptyNewLaunch),
        palette,
    )
    .on_press(props.on_new_launch);

    EmptyCard::new(
        i18n::t(Key::EmptyTitle),
        i18n::t(Key::EmptyDescription),
        logo,
        palette,
    )
    .action(new_terminal)
    .action(new_launch)
    .into()
}

/// Return the logo handle, created once. `Handle::from_bytes` gives every
/// new handle a new id, so a handle built on each render looks like a new
/// image to iced: it reloads the logo every frame and the logo flickers.
fn logo_handle() -> &'static image::Handle {
    static LOGO: OnceLock<image::Handle> = OnceLock::new();

    LOGO.get_or_init(|| image::Handle::from_bytes(icons::LOGO))
}

#[cfg(test)]
mod tests {
    use iced::keyboard::key::Named;
    use iced::widget::container;
    use iced::{Element, Length, Point, Size, Theme, event, mouse};
    use iced_test::selector::Candidate;
    use iced_test::{Simulator, simulator};
    use otty_ui_kit::components::screen_area::ScreenArea;
    use otty_ui_kit::size;

    use super::{EmptyStateProps, view};
    use crate::shortcuts::AppShortcut;
    use crate::testing::temp_dir::TempDir;
    use crate::theme::{ThemeManager, ThemeProps};

    /// A main area with room for the card.
    const ROOMY: Size = Size::new(900.0, 700.0);
    /// A main area narrower than the card can get and lower than the card:
    /// both panels dragged wide in a low window can leave this.
    const SMALL_MAIN: Size = Size::new(278.0, 200.0);
    /// Far enough to reach the end of the card.
    const FAR: f32 = -10_000.0;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Message {
        NewTerminal,
        NewLaunch,
    }

    fn props(themes: &ThemeManager) -> EmptyStateProps<'_, Message> {
        EmptyStateProps {
            theme: ThemeProps::new(themes.current()),
            on_new_terminal: Message::NewTerminal,
            on_new_launch: Message::NewLaunch,
        }
    }

    /// The card in the area it has in the app.
    fn card(themes: &ThemeManager) -> Element<'_, Message> {
        ScreenArea::new(0, view(props(themes)), themes.current().ui()).into()
    }

    fn card_in_area(
        themes: &ThemeManager,
        area: Size,
    ) -> Simulator<'_, Message> {
        Simulator::with_size(iced::Settings::default(), area, card(themes))
    }

    /// Scrolls after a frame, as in the running app: a scroller that has never
    /// been drawn takes the first wheel for itself. As in the tables, a
    /// negative number scrolls right or down.
    fn wheel(ui: &mut Simulator<'_, Message>, x: f32, y: f32) {
        ui.point_at(Point::new(10.0, 10.0));
        let _ = ui.simulate([iced::Event::Window(
            iced::window::Event::RedrawRequested(iced::time::Instant::now()),
        )]);
        let _ =
            ui.simulate([iced::Event::Mouse(mouse::Event::WheelScrolled {
                delta: mouse::ScrollDelta::Pixels { x, y },
            })]);
    }

    /// How far the page is scrolled, how large its content is and how large
    /// the part that shows is.
    fn scroller(ui: &mut Simulator<'_, Message>) -> (Point, Size, Size) {
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
    fn given_empty_state_when_viewed_then_title_and_tiles_show() {
        let themes = ThemeManager::new();
        let mut ui = simulator(card(&themes));

        assert!(ui.find("Start a session").is_ok());
        assert!(ui.find("New terminal").is_ok());
        assert!(ui.find("New launch…").is_ok());
        assert!(ui.find("Terminal in a folder…").is_err());
    }

    #[test]
    fn given_empty_state_when_new_terminal_pressed_then_it_is_sent() {
        let themes = ThemeManager::new();
        let mut ui = simulator(card(&themes));

        ui.click("New terminal").expect("tile should be visible");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::NewTerminal]);
    }

    #[test]
    fn given_empty_state_when_new_launch_pressed_then_it_is_sent() {
        let themes = ThemeManager::new();
        let mut ui = simulator(card(&themes));

        ui.click("New launch…").expect("tile should be visible");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::NewLaunch]);
    }

    /// With room for it the drawing must not change: it is the old layout, a
    /// card in a centered container. The odd sizes put the card on half a pixel.
    #[test]
    fn with_room_the_card_draws_as_in_a_centered_container() {
        let themes = ThemeManager::new();
        let folder = TempDir::new("start-card");

        for (index, area) in [
            ROOMY,
            Size::new(901.0, 701.0),
            Size::new(size::SCREEN_MIN, 500.0),
        ]
        .into_iter()
        .enumerate()
        {
            let reference = folder.path().join(format!("area-{index}"));
            let plain = container(view(props(&themes)))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill);

            let plain_matches =
                Simulator::with_size(iced::Settings::default(), area, plain)
                    .snapshot(&Theme::Dark)
                    .expect("the card should draw")
                    .matches_hash(&reference)
                    .expect("reference should be readable");
            let page_matches = card_in_area(&themes, area)
                .snapshot(&Theme::Dark)
                .expect("the card should draw")
                .matches_hash(&reference)
                .expect("reference should be readable");

            assert!(plain_matches);
            assert!(page_matches, "at {area:?}");
        }
    }

    #[test]
    fn with_room_nothing_scrolls() {
        let themes = ThemeManager::new();
        let mut ui = card_in_area(&themes, ROOMY);

        let (_, content, showing) = scroller(&mut ui);
        wheel(&mut ui, FAR, FAR);
        let (offset, _, _) = scroller(&mut ui);

        assert_eq!(content, showing);
        assert_eq!(offset, Point::ORIGIN);
    }

    /// The card keeps its minimum, starts at its top left corner and moves
    /// each way on its own.
    #[test]
    fn in_a_small_area_the_card_scrolls_both_ways() {
        let themes = ThemeManager::new();
        let mut ui = card_in_area(&themes, SMALL_MAIN);

        let (start, content, showing) = scroller(&mut ui);
        wheel(&mut ui, 0.0, -20.0);
        let (down, _, _) = scroller(&mut ui);
        wheel(&mut ui, -20.0, 0.0);
        let (right, _, _) = scroller(&mut ui);
        wheel(&mut ui, FAR, FAR);
        let (end, _, _) = scroller(&mut ui);

        assert_eq!(content.width, size::SCREEN_MIN);
        assert!(content.height > showing.height);
        assert_eq!(start, Point::ORIGIN);
        assert_eq!(down, Point::new(0.0, 20.0));
        assert_eq!(right, Point::new(20.0, 20.0));
        assert_eq!(
            end,
            Point::new(
                content.width - showing.width,
                content.height - showing.height
            )
        );
    }

    #[test]
    fn in_a_small_area_each_tile_can_be_reached_and_pressed() {
        let themes = ThemeManager::new();

        for (label, expected) in [
            ("New terminal", Message::NewTerminal),
            ("New launch…", Message::NewLaunch),
        ] {
            let mut ui = card_in_area(&themes, SMALL_MAIN);

            wheel(&mut ui, 0.0, FAR);
            ui.click(label).expect("the tile should be in reach");

            let sent: Vec<Message> = ui.into_messages().collect();
            assert_eq!(sent, vec![expected]);
        }
    }

    /// The right end of the card, cut off at first, can be reached too.
    #[test]
    fn in_a_small_area_the_shortcut_hint_can_be_reached_and_pressed() {
        let themes = ThemeManager::new();
        let hint = AppShortcut::NewTerminal.label();
        let mut ui = card_in_area(&themes, SMALL_MAIN);

        wheel(&mut ui, FAR, FAR);
        ui.click(hint.as_str())
            .expect("the hint should be in reach");

        let sent: Vec<Message> = ui.into_messages().collect();
        assert_eq!(sent, vec![Message::NewTerminal]);
    }

    /// The app's shortcuts only get keys no widget took, so the card must not
    /// take any, whether it fits or not.
    #[test]
    fn the_card_takes_no_key() {
        let themes = ThemeManager::new();

        for area in [ROOMY, SMALL_MAIN] {
            let mut ui = card_in_area(&themes, area);

            for key in [
                Named::Tab,
                Named::Enter,
                Named::Space,
                Named::PageDown,
                Named::ArrowDown,
                Named::Escape,
            ] {
                assert_eq!(
                    ui.tap_key(key),
                    event::Status::Ignored,
                    "{key:?} at {area:?}"
                );
            }

            assert_eq!(ui.into_messages().count(), 0);
        }
    }

    #[test]
    fn keys_do_not_move_a_scrolled_card() {
        let themes = ThemeManager::new();
        let mut ui = card_in_area(&themes, SMALL_MAIN);

        wheel(&mut ui, -20.0, -20.0);
        let (before, _, _) = scroller(&mut ui);
        for key in [
            Named::PageDown,
            Named::PageUp,
            Named::ArrowDown,
            Named::Home,
            Named::End,
        ] {
            let _ = ui.tap_key(key);
        }
        let (after, _, _) = scroller(&mut ui);

        assert_eq!(before, Point::new(20.0, 20.0));
        assert_eq!(after, before);
    }
}

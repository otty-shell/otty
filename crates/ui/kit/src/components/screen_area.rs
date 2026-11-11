//! `ScreenArea`: the one place that says how a screen of the main area deals
//! with too little room. It keeps a minimum width and scrolls.
//!
//! A `scrollable` that scrolls both ways lays out its content with no upper
//! limit, so a `Fill` inside it can not fill anything and text does not wrap.
//! It does hand down the lower limit it got itself, so [`ScreenArea`] gives it
//! the whole room as that limit and a private widget inside the `scrollable`
//! reads it back. That widget lays out the screen at the room's width or at
//! [`size::SCREEN_MIN`], whichever is more, and at the room's height while the
//! screen is lower than that.
//!
//! To learn whether it is lower, it lays out the screen with no lower limit
//! first, so a screen that fits is laid out twice. There is one area per screen
//! and no area inside another, so this does not multiply. `responsive`, which
//! `DataTable` uses, does not fit here: it builds its content again for each
//! width, and a screen holds elements it was handed once.

use iced::advanced::layout::{self, Layout, Limits};
use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Shell, Widget, overlay, renderer};
use iced::widget::{container, scrollable};
use iced::{
    Alignment, Element, Event, Length, Rectangle, Renderer, Size, Theme,
    Vector, mouse,
};

use super::scrollbar;
use crate::palette::UiPalette;
use crate::size;

/// One screen of the main area, in the one scroller it needs. What is inside
/// does not matter:
///
/// - In a room of at least [`size::SCREEN_MIN`] the screen takes the room's
///   width. Below that it keeps that width and the area scrolls sideways.
/// - A screen lower than the room gets the room's height, so a form's `Fill`
///   works and its footer lands at the bottom. A taller one scrolls down.
/// - A screen that does not fill the room, such as a card, is centered both
///   ways, as long as it fits.
/// - The bars are the kit's thin ones, which float and take no room.
///
/// `key` says which screen this is. iced keeps the state of a widget for
/// whatever comes next at the same place, so without it a new screen would open
/// at the scroll position and the focus of the old one. A screen built again
/// with the same key keeps both; another key starts afresh at the top left. A
/// keyed column would not do this: with one child it only compares the keys and
/// still hands the old state on.
///
/// Not for what scrolls on its own and needs a height it can fill: a terminal
/// or a table with its sticky header.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use iced::widget::text;
/// use otty_ui_kit::components::screen_area::ScreenArea;
/// use otty_ui_kit::palette::UiPalette;
///
/// let palette = UiPalette::otty_dark();
/// let screen: Element<'_, ()> =
///     ScreenArea::new("settings", text("Settings"), &palette).into();
/// ```
pub struct ScreenArea<'a, Message, Key> {
    key: Key,
    scroller: Element<'a, Message>,
}

impl<'a, Message: 'a, Key> ScreenArea<'a, Message, Key> {
    /// Puts `content`, the screen named `key`, in the area.
    pub fn new(
        key: Key,
        content: impl Into<Element<'a, Message>>,
        palette: &UiPalette,
    ) -> Self {
        let centered = container(content)
            .center_x(Length::Fill)
            .align_y(Alignment::Center);

        let scroller = scrollable(Room {
            content: centered.into(),
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .direction(scrollbar::both())
        .style(scrollbar::style(palette))
        .into();

        Self { key, scroller }
    }
}

impl<Message, Key> Widget<Message, Theme, Renderer>
    for ScreenArea<'_, Message, Key>
where
    Key: Clone + PartialEq + 'static,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Key>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(self.key.clone())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.scroller)]
    }

    fn diff(&self, tree: &mut Tree) {
        if *tree.state.downcast_ref::<Key>() == self.key {
            tree.diff_children(std::slice::from_ref(&self.scroller));
            return;
        }

        tree.state = self.state();
        tree.children = self.children();
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &Limits,
    ) -> layout::Node {
        self.scroller.as_widget_mut().layout(
            &mut tree.children[0],
            renderer,
            &Limits::new(limits.max(), limits.max()),
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.scroller.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.scroller.as_widget_mut().operate(
            &mut tree.children[0],
            layout,
            renderer,
            operation,
        );
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.scroller.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.scroller.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.scroller.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message, Key> From<ScreenArea<'a, Message, Key>>
    for Element<'a, Message>
where
    Message: 'a,
    Key: Clone + PartialEq + 'static,
{
    fn from(value: ScreenArea<'a, Message, Key>) -> Self {
        Element::new(value)
    }
}

/// The content of the scroller: gives the screen the room the scroller handed
/// down as its lower limit, or the minimum width. See the module documentation.
struct Room<'a, Message> {
    content: Element<'a, Message>,
}

impl<Message> Widget<Message, Theme, Renderer> for Room<'_, Message> {
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Shrink, Length::Shrink)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &Limits,
    ) -> layout::Node {
        let content = self.content.as_widget_mut();
        let tree = &mut tree.children[0];
        let width = limits.min().width.max(size::SCREEN_MIN);
        let room = limits.min().height;

        // First with no lower limit, to learn the height the screen needs.
        let needed = content.layout(
            tree,
            renderer,
            &Limits::with_compression(
                Size::ZERO,
                Size::new(width, limits.max().height),
                Size::new(false, limits.compression().height),
            ),
        );
        if needed.size().height >= room {
            return needed;
        }

        content.layout(
            tree,
            renderer,
            &Limits::new(Size::new(0.0, room), Size::new(width, room)),
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content.as_widget_mut().operate(
            &mut tree.children[0],
            layout,
            renderer,
            operation,
        );
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message: 'a> From<Room<'a, Message>> for Element<'a, Message> {
    fn from(value: Room<'a, Message>) -> Self {
        Element::new(value)
    }
}

#[cfg(test)]
mod tests {
    use iced::Point;
    use iced::advanced::widget::operation;
    use iced::keyboard::key::Named;
    use iced::keyboard::{self, Modifiers};
    use iced::widget::{Space, button, column, row, text, text_input};
    use iced_test::Simulator;
    use iced_test::runtime::user_interface::{Cache, UserInterface};
    use iced_test::selector::{Candidate, Selector};

    use super::*;
    use crate::test_support::shift_wheel_notch;

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Pressed,
        Typed(String),
    }

    /// Larger than the minimum both ways, for the content of [`tall`].
    const ROOMY: Size = Size::new(900.0, 800.0);
    /// Narrower than the minimum and lower than the content of [`tall`].
    const SMALL: Size = Size::new(300.0, 200.0);
    /// Far enough to reach the end of any page here.
    const FAR: f32 = -10_000.0;
    /// The key of most tests here.
    const SCREEN: u32 = 1;

    fn in_area(
        content: impl Into<Element<'static, Message>>,
        window: Size,
    ) -> Simulator<'static, Message> {
        Simulator::with_size(
            iced::Settings::default(),
            window,
            ScreenArea::new(SCREEN, content, &UiPalette::otty_dark()),
        )
    }

    /// `rows` lines between a first line as wide as the screen and a last line
    /// with a button at its right end.
    fn tall(rows: usize) -> Element<'static, Message> {
        let mut screen = column![text("first").width(Length::Fill)];

        for index in 0..rows {
            screen = screen.push(text(format!("line {index}")));
        }

        screen
            .push(row![
                Space::new().width(Length::Fill),
                button(text("Last")).on_press(Message::Pressed),
            ])
            .width(Length::Fill)
            .into()
    }

    /// A screen that fills the room: a line on top and one at the bottom.
    fn filling() -> Element<'static, Message> {
        column![
            text("top"),
            Space::new().height(Length::Fill),
            text("bottom"),
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    /// Scrolls under the pointer. As in the tables, a negative number scrolls
    /// right or down.
    fn wheel(ui: &mut Simulator<'_, Message>, x: f32, y: f32) {
        ui.point_at(Point::new(10.0, 10.0));
        let _ = ui.simulate([Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Pixels { x, y },
        })]);
    }

    /// How far the area is scrolled, how large its content is and how large
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
        .expect("the area should have a scroller")
    }

    #[test]
    fn given_more_room_than_the_minimum_then_the_screen_takes_it_and_nothing_scrolls()
     {
        let mut ui = in_area(tall(0), ROOMY);

        let first = ui.find("first").expect("the line").bounds();
        let (_, content, showing) = scroller(&mut ui);
        wheel(&mut ui, FAR, FAR);
        let (offset, _, _) = scroller(&mut ui);

        assert_eq!(first.width, ROOMY.width);
        assert_eq!(content, showing, "no bar should show");
        assert_eq!(offset, Point::ORIGIN);
    }

    #[test]
    fn given_exactly_the_minimum_then_nothing_scrolls_sideways() {
        let mut ui = in_area(tall(0), Size::new(size::SCREEN_MIN, 800.0));

        let (_, content, showing) = scroller(&mut ui);

        assert_eq!(content, showing);
    }

    #[test]
    fn given_less_room_than_the_minimum_then_the_screen_keeps_it_and_a_far_control_is_in_reach()
     {
        let mut ui = in_area(tall(0), Size::new(SMALL.width, 800.0));

        let first = ui.find("first").expect("the line").bounds();
        wheel(&mut ui, FAR, 0.0);
        let (offset, content, showing) = scroller(&mut ui);
        ui.click("Last").expect("the button should be in reach");

        let sent: Vec<Message> = ui.into_messages().collect();
        assert_eq!(first.width, size::SCREEN_MIN);
        assert_eq!(content.height, showing.height, "no bar down");
        assert_eq!(offset.x, size::SCREEN_MIN - SMALL.width);
        assert_eq!(offset.y, 0.0);
        assert_eq!(sent, vec![Message::Pressed]);
    }

    #[test]
    fn given_a_screen_lower_than_the_room_then_a_fill_reaches_the_bottom() {
        let mut ui = in_area(filling(), ROOMY);

        let top = ui.find("top").expect("top").bounds();
        let bottom = ui.find("bottom").expect("bottom").bounds();
        let (_, content, showing) = scroller(&mut ui);

        assert_eq!(top.y, 0.0);
        assert_eq!(bottom.y + bottom.height, ROOMY.height);
        assert_eq!(content, showing);
    }

    #[test]
    fn given_a_screen_taller_than_the_room_then_a_fill_takes_nothing_and_it_scrolls_to_its_end()
     {
        let screen = column![
            text("top"),
            Space::new().height(Length::Fill),
            Space::new().height(1000.0),
            text("bottom"),
        ]
        .height(Length::Fill);
        let mut ui = in_area(screen, Size::new(800.0, 300.0));

        let hidden = ui.find("bottom").expect("bottom");
        wheel(&mut ui, 0.0, FAR);
        let shown = ui.find("bottom").expect("bottom");

        assert_eq!(hidden.visible_bounds(), None);
        let shown = shown.visible_bounds().expect("it should come into view");
        assert!(shown.y + shown.height <= 300.0);
    }

    #[test]
    fn given_a_small_screen_with_room_then_it_is_centered_both_ways() {
        let card = container(text("card"))
            .width(Length::Fixed(200.0))
            .height(Length::Fixed(100.0));
        let mut ui = in_area(card, ROOMY);

        let line = ui.find("card").expect("the text").bounds();
        let (_, content, showing) = scroller(&mut ui);

        assert_eq!(line.x, (ROOMY.width - 200.0) / 2.0);
        assert_eq!(line.y, (ROOMY.height - 100.0) / 2.0);
        assert_eq!(content, showing);
    }

    #[test]
    fn given_a_small_screen_below_the_minimum_then_it_is_centered_in_the_minimum()
     {
        let card = container(text("card")).width(Length::Fixed(200.0));
        let mut ui = in_area(card, SMALL);

        let line = ui.find("card").expect("the text").bounds();

        assert_eq!(line.x, (size::SCREEN_MIN - 200.0) / 2.0);
    }

    #[test]
    fn given_a_screen_too_tall_then_it_starts_at_its_top() {
        let card = container(tall(30)).width(Length::Fixed(200.0));
        let mut ui = in_area(card, Size::new(800.0, 200.0));

        let first = ui.find("first").expect("the line").bounds();

        assert_eq!(first.y, 0.0);
    }

    #[test]
    fn given_a_small_room_when_scrolled_then_each_direction_moves_on_its_own() {
        let mut ui = in_area(tall(30), SMALL);

        wheel(&mut ui, 0.0, -20.0);
        let (down, _, _) = scroller(&mut ui);
        wheel(&mut ui, -20.0, 0.0);
        let (right, _, _) = scroller(&mut ui);

        assert_eq!(down, Point::new(0.0, 20.0));
        assert_eq!(right, Point::new(20.0, 20.0));
    }

    #[test]
    fn given_a_small_room_when_shift_and_the_wheel_are_used_then_it_moves_sideways()
     {
        let mut ui = in_area(tall(30), SMALL);

        let _ = ui.simulate([Event::Keyboard(
            keyboard::Event::ModifiersChanged(Modifiers::SHIFT),
        )]);
        ui.point_at(Point::new(10.0, 10.0));
        let _ = ui.simulate([Event::Mouse(mouse::Event::WheelScrolled {
            delta: shift_wheel_notch(),
        })]);
        let (offset, _, _) = scroller(&mut ui);

        assert!(offset.x > 0.0, "{offset:?}");
        assert_eq!(offset.y, 0.0);
    }

    #[test]
    fn given_a_small_room_when_scrolled_to_the_far_corner_then_it_is_in_view() {
        let mut ui = in_area(tall(30), SMALL);

        let hidden = ui.find("Last").expect("the button");
        wheel(&mut ui, FAR, FAR);
        let shown = ui.find("Last").expect("the button");

        assert_eq!(hidden.visible_bounds(), None);
        let shown = shown.visible_bounds().expect("it should come into view");
        assert!(shown.x + shown.width <= SMALL.width);
        assert!(shown.y + shown.height <= SMALL.height);
    }

    #[test]
    fn given_a_screen_that_overflows_when_its_bars_show_then_the_screen_does_not_move()
     {
        let mut fits = in_area(tall(5), Size::new(SMALL.width, 800.0));
        let mut overflows = in_area(tall(5), Size::new(SMALL.width, 60.0));

        let fits = fits.find("first").expect("the line").bounds();
        let overflows = overflows.find("first").expect("the line").bounds();

        assert_eq!(fits.x, overflows.x);
        assert_eq!(fits.width, overflows.width);
    }

    #[test]
    fn the_area_takes_no_key() {
        for window in [ROOMY, SMALL] {
            let mut ui = in_area(tall(30), window);

            wheel(&mut ui, -20.0, -20.0);
            let (before, _, _) = scroller(&mut ui);
            for key in [
                Named::Tab,
                Named::Enter,
                Named::Space,
                Named::PageDown,
                Named::PageUp,
                Named::ArrowDown,
                Named::Home,
                Named::End,
                Named::Escape,
            ] {
                assert_eq!(
                    ui.tap_key(key),
                    iced::event::Status::Ignored,
                    "{key:?} in {window:?}"
                );
            }
            let (after, _, _) = scroller(&mut ui);

            assert_eq!(after, before, "in {window:?}");
        }
    }

    /// One frame after another, as the running app builds them: the widget
    /// state carries over from one to the next.
    struct Frames {
        renderer: Renderer,
        cache: Option<Cache>,
    }

    impl Frames {
        fn new() -> Self {
            use iced_test::core::renderer::Headless;

            let renderer =
                iced_test::runtime::futures::futures::executor::block_on(
                    <Renderer as Headless>::new(
                        iced::Font::DEFAULT,
                        iced::Pixels(16.0),
                        None,
                    ),
                )
                .expect("a headless renderer");

            Self {
                renderer,
                cache: Some(Cache::default()),
            }
        }

        /// Builds a frame of `screen`, gives it `events` with the pointer in
        /// the area, and returns how far the outer scroller is scrolled and
        /// what the screen sent.
        fn frame(
            &mut self,
            screen: Element<'static, Message>,
            events: &[Event],
        ) -> (Vector, Vec<Message>) {
            let mut ui = UserInterface::build(
                screen,
                SMALL,
                self.cache.take().expect("the frames keep their cache"),
                &mut self.renderer,
            );
            let mut messages = Vec::new();
            let _ = ui.update(
                events,
                mouse::Cursor::Available(Point::new(40.0, 10.0)),
                &mut self.renderer,
                &mut iced::advanced::clipboard::Null,
                &mut messages,
            );

            let mut find = (|candidate: Candidate<'_>| match candidate {
                Candidate::Scrollable { translation, .. } => Some(translation),
                _ => None,
            })
            .find();
            ui.operate(&self.renderer, &mut operation::black_box(&mut find));
            let offset = match find.finish() {
                operation::Outcome::Some(Some(offset)) => offset,
                _ => panic!("the area should have a scroller"),
            };

            self.cache = Some(ui.into_cache());

            (offset, messages)
        }
    }

    /// A screen of [`tall`] named `key` with `rows` lines in a frame.
    fn tall_screen(key: u32, rows: usize) -> Element<'static, Message> {
        ScreenArea::new(key, tall(rows), &UiPalette::otty_dark()).into()
    }

    fn wheel_event(x: f32, y: f32) -> Event {
        Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Pixels { x, y },
        })
    }

    #[test]
    fn given_another_key_when_built_then_it_starts_at_the_top_left() {
        let mut frames = Frames::new();
        let top = Vector::new(0.0, 0.0);

        let _ = frames.frame(tall_screen(1, 30), &[]);
        let (scrolled, _) =
            frames.frame(tall_screen(1, 30), &[wheel_event(-30.0, -50.0)]);
        let (other, _) = frames.frame(tall_screen(2, 30), &[]);
        let (other_scrolled, _) =
            frames.frame(tall_screen(2, 30), &[wheel_event(-5.0, -5.0)]);
        let (first_again, _) = frames.frame(tall_screen(1, 30), &[]);

        assert_eq!(scrolled, Vector::new(30.0, 50.0));
        assert_eq!(other, top);
        assert_eq!(other_scrolled, Vector::new(5.0, 5.0));
        assert_eq!(first_again, top, "the area keeps no memory of a screen");
    }

    /// The same screen built again keeps its place, also when what it holds
    /// changes, as with a validation message or a new row.
    #[test]
    fn given_the_same_key_when_built_again_then_it_keeps_its_place() {
        let mut frames = Frames::new();

        let _ = frames.frame(tall_screen(1, 30), &[]);
        let (scrolled, _) =
            frames.frame(tall_screen(1, 30), &[wheel_event(-30.0, -50.0)]);
        let (same, _) = frames.frame(tall_screen(1, 30), &[]);
        let (more_rows, _) = frames.frame(tall_screen(1, 40), &[]);

        assert_eq!(scrolled, Vector::new(30.0, 50.0));
        assert_eq!(same, scrolled);
        assert_eq!(more_rows, scrolled);
    }

    /// Two kinds of key are two screens too.
    #[test]
    fn given_a_key_of_another_type_when_built_then_it_starts_at_the_top_left() {
        let mut frames = Frames::new();

        let _ = frames.frame(tall_screen(1, 30), &[]);
        let _ = frames.frame(tall_screen(1, 30), &[wheel_event(-30.0, -50.0)]);
        let other: Element<'static, Message> =
            ScreenArea::new("other", tall(30), &UiPalette::otty_dark()).into();
        let (other, _) = frames.frame(other, &[]);

        assert_eq!(other, Vector::new(0.0, 0.0));
    }

    /// A screen with an input at the top left.
    fn screen_with_input(key: u32) -> Element<'static, Message> {
        let input = text_input("", "").on_input(Message::Typed);
        let screen = column![input, tall(30)].width(Length::Fill);

        ScreenArea::new(key, screen, &UiPalette::otty_dark()).into()
    }

    fn press_events() -> [Event; 2] {
        [
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        ]
    }

    fn typed_key() -> Event {
        Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Character("n".into()),
            modified_key: keyboard::Key::Character("n".into()),
            physical_key: keyboard::key::Physical::Code(
                keyboard::key::Code::KeyN,
            ),
            location: keyboard::Location::Standard,
            modifiers: Modifiers::empty(),
            text: Some("n".into()),
            repeat: false,
        })
    }

    /// The input is pressed on one screen. Typing goes on while that screen is
    /// built again, and it does not reach an input of the next screen.
    #[test]
    fn the_focus_of_an_input_stays_with_its_screen() {
        let mut frames = Frames::new();

        let _ = frames.frame(screen_with_input(1), &[]);
        let _ = frames.frame(screen_with_input(1), &press_events());
        let (_, same) = frames.frame(screen_with_input(1), &[typed_key()]);
        let (_, other) = frames.frame(screen_with_input(2), &[typed_key()]);

        assert_eq!(same, vec![Message::Typed(String::from("n"))]);
        assert!(other.is_empty(), "{other:?}");
    }
}

//! `press_away`: tells when a click on some content sent nothing, such as a
//! click on an empty part of a window, or on a widget that takes the press
//! but sends no message. An app closes its open menus then: the menus of
//! the kit have no layer of their own that takes a press away from them.

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell, overlay, renderer};
use iced::{
    Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector, mouse,
};

/// The content and the message of a click on it that sent nothing.
struct PressAway<'a, Message> {
    content: Element<'a, Message>,
    on_away: Message,
}

impl<Message: Clone> Widget<Message, Theme, Renderer>
    for PressAway<'_, Message>
{
    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[self.content.as_widget()]);
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content.as_widget_mut().layout(
            &mut tree.children[0],
            renderer,
            limits,
        )
    }

    /// Passes `event` to the content and watches what it sends. A press
    /// over the content starts a click; its release ends it, and sends
    /// `on_away` when neither of them made the content send a message.
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
        let mut messages = Vec::new();
        let mut content_shell = Shell::new(&mut messages);
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            &mut content_shell,
            viewport,
        );
        let has_sent = !content_shell.is_empty();
        shell.merge(content_shell, std::convert::identity);

        let state = tree.state.downcast_mut::<State>();
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(_))
                if cursor.is_over(layout.bounds()) =>
            {
                state.click = Some(Click { has_sent });
            },
            Event::Mouse(mouse::Event::ButtonReleased(_)) => {
                let click = state.click.take();
                if click.is_some_and(|click| !click.has_sent) && !has_sent {
                    shell.publish(self.on_away.clone());
                }
            },
            _ => {},
        }
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

/// What a [`PressAway`] remembers between events: the click under way.
#[derive(Default)]
struct State {
    click: Option<Click>,
}

/// A click whose press landed on the content.
#[derive(Clone, Copy)]
struct Click {
    /// Whether the press made the content send a message.
    has_sent: bool,
}

/// Shows `content` and sends `on_away` after a click on it, a press of any
/// mouse button and its release, that sent nothing: a click on an empty
/// part, or on a widget that takes the press but sends no message, such as
/// a text field or a scrollbar. A click that sends a message, on a press
/// or on its release, does not send it.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use iced::widget::text;
/// use otty_ui_kit::components::press_away::press_away;
///
/// #[derive(Clone)]
/// enum Message {
///     CloseMenus,
/// }
///
/// let window: Element<'_, Message> =
///     press_away(text("The window"), Message::CloseMenus);
/// ```
pub fn press_away<'a, Message: Clone + 'a>(
    content: impl Into<Element<'a, Message>>,
    on_away: Message,
) -> Element<'a, Message> {
    Element::new(PressAway {
        content: content.into(),
        on_away,
    })
}

#[cfg(test)]
mod tests {
    use iced::widget::{Space, button, column, container, mouse_area};
    use iced::{Point, mouse};
    use iced_test::simulator;

    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Pressed,
        Down,
        Away,
    }

    /// A button, a mouse area that sends on press, and an empty part, in
    /// a column 100 wide; each row 40 high.
    fn window<'a>() -> Element<'a, Message> {
        let content = column![
            button(Space::new().width(100).height(40))
                .on_press(Message::Pressed),
            mouse_area(Space::new().width(100).height(40))
                .on_press(Message::Down),
            container(Space::new().width(100).height(40)),
        ];

        press_away(content, Message::Away)
    }

    /// What a click of `button` at `point` sends.
    fn clicked_at(point: Point, button: mouse::Button) -> Vec<Message> {
        let mut ui = simulator(window());

        ui.point_at(point);
        let _ = ui.simulate([
            Event::Mouse(mouse::Event::CursorMoved { position: point }),
            Event::Mouse(mouse::Event::ButtonPressed(button)),
            Event::Mouse(mouse::Event::ButtonReleased(button)),
        ]);

        ui.into_messages().collect()
    }

    #[test]
    fn a_click_on_an_empty_part_is_a_press_away() {
        let empty = Point::new(50.0, 100.0);

        for button in [mouse::Button::Left, mouse::Button::Right] {
            assert_eq!(clicked_at(empty, button), [Message::Away]);
        }
    }

    #[test]
    fn a_click_that_sends_a_message_is_no_press_away() {
        assert_eq!(
            clicked_at(Point::new(50.0, 20.0), mouse::Button::Left),
            [Message::Pressed]
        );
        assert_eq!(
            clicked_at(Point::new(50.0, 60.0), mouse::Button::Left),
            [Message::Down]
        );
    }
}

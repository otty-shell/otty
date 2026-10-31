//! Reports Backspace pressed in an empty, focused text input.
//!
//! `iced` 0.14 text inputs send nothing when Backspace is pressed on an empty
//! value, so [`TokenField`](super::token_field::TokenField) wraps its input in
//! this widget.

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell, overlay, renderer, text};
use iced::keyboard::key::Named;
use iced::widget::text_input;
use iced::{
    Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector, keyboard,
    mouse,
};

/// Wraps a text input and publishes a message on Backspace when the input is
/// focused and its value is empty.
pub(super) struct BackspaceGuard<'a, Message> {
    content: Element<'a, Message>,
    is_empty: bool,
    on_backspace: Message,
}

impl<'a, Message> BackspaceGuard<'a, Message> {
    /// Wraps `input`, a [`text_input`] whose current value is empty when
    /// `is_empty` is true.
    pub(super) fn new(
        input: impl Into<Element<'a, Message>>,
        is_empty: bool,
        on_backspace: Message,
    ) -> Self {
        Self {
            content: input.into(),
            is_empty,
            on_backspace,
        }
    }
}

impl<Message: Clone> Widget<Message, Theme, Renderer>
    for BackspaceGuard<'_, Message>
{
    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
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

    fn tag(&self) -> tree::Tag {
        tree::Tag::stateless()
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
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
        let is_backspace = matches!(
            event,
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(Named::Backspace),
                ..
            })
        );
        let is_focused = tree.children[0]
            .state
            .downcast_ref::<text_input::State<
                <Renderer as text::Renderer>::Paragraph,
            >>()
            .is_focused();
        if is_backspace && self.is_empty && is_focused {
            shell.publish(self.on_backspace.clone());
            shell.capture_event();

            return;
        }

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

impl<'a, Message: Clone + 'a> From<BackspaceGuard<'a, Message>>
    for Element<'a, Message>
{
    fn from(value: BackspaceGuard<'a, Message>) -> Self {
        Element::new(value)
    }
}

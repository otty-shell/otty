//! Reports named keys pressed in a focused text input.
//!
//! `iced` 0.14 text inputs send nothing for Backspace on an empty value, for
//! ↑ and ↓, they only drop focus on Esc, and they do not say when they get
//! the focus, and they leave the cursor in place when the app replaces the
//! text (see [`InputKeys::follow_text`]). [`TokenField`], [`CommandPalette`] and [`TagSearchField`] wrap
//! their input in [`InputKeys`] to turn those into messages.
//!
//! [`TokenField`]: super::token_field::TokenField
//! [`CommandPalette`]: super::command_palette::CommandPalette
//! [`TagSearchField`]: super::tag_search_field::TagSearchField

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell, overlay, renderer, text};
use iced::keyboard::key::Named;
use iced::widget::text_input;
use iced::{
    Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector, keyboard,
    mouse,
};

/// Wraps a text input and publishes the message bound to a named key when
/// that key is pressed while the input is focused. The key press then does
/// not reach the input.
pub(super) struct InputKeys<'a, Message> {
    content: Element<'a, Message>,
    bindings: Vec<(Named, Message)>,
    on_focus: Option<Message>,
    follow_text: Option<String>,
}

impl<'a, Message> InputKeys<'a, Message> {
    /// Wraps `input`, which must be a [`text_input`], with no bound keys.
    pub(super) fn new(input: impl Into<Element<'a, Message>>) -> Self {
        Self {
            content: input.into(),
            bindings: Vec::new(),
            on_focus: None,
            follow_text: None,
        }
    }

    /// Moves the cursor to the end of the text when the app replaces `text`
    /// from outside the input (a suggestion applied, a field cleared). `iced`
    /// 0.14 leaves the cursor where it was, so the next letter would land
    /// before the text. Typing, pasting and moving the cursor by keys never
    /// move it, so an edit in the middle of the text stays there.
    pub(super) fn follow_text(mut self, text: &str) -> Self {
        self.follow_text = Some(text.to_owned());
        self
    }

    /// Sends `message` when `key` is pressed in the focused input.
    pub(super) fn on_key(mut self, key: Named, message: Message) -> Self {
        self.bindings.push((key, message));
        self
    }

    /// Sends `message` when the input gets the focus. A focus given by the
    /// app (`iced::widget::operation::focus`) is reported on the next event.
    pub(super) fn on_focus(mut self, message: Message) -> Self {
        self.on_focus = Some(message);
        self
    }

    /// Returns the message bound to the key pressed in `event`.
    fn bound_message(&self, event: &Event) -> Option<&Message> {
        let Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(pressed),
            ..
        }) = event
        else {
            return None;
        };

        self.bindings
            .iter()
            .find(|(key, _)| key == pressed)
            .map(|(_, message)| message)
    }
}

impl<Message: Clone> Widget<Message, Theme, Renderer>
    for InputKeys<'_, Message>
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
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));

        let Some(text) = &self.follow_text else {
            return;
        };
        let state = tree.state.downcast_mut::<State>();
        let replaced_outside = state.text != *text && !state.input_edited;
        state.text.clone_from(text);
        state.input_edited = false;

        if replaced_outside {
            input_state_mut(tree).move_cursor_to_end();
        }
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
        if input_is_focused(tree)
            && let Some(message) = self.bound_message(event)
        {
            tree.state.downcast_mut::<State>().input_edited = false;
            shell.publish(message.clone());
            shell.capture_event();

            return;
        }

        let is_focused = input_is_focused(tree);
        let state = tree.state.downcast_mut::<State>();
        match event {
            Event::Keyboard(keyboard::Event::KeyPressed { .. })
            | Event::InputMethod(_)
                if is_focused =>
            {
                state.input_edited = true;
            },
            Event::Mouse(mouse::Event::ButtonPressed(_)) => {
                state.input_edited = false;
            },
            _ => {},
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

        let is_focused = input_is_focused(tree);
        let state = tree.state.downcast_mut::<State>();
        if is_focused
            && !state.was_focused
            && let Some(message) = &self.on_focus
        {
            shell.publish(message.clone());
        }
        state.was_focused = is_focused;
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

impl<'a, Message: Clone + 'a> From<InputKeys<'a, Message>>
    for Element<'a, Message>
{
    fn from(value: InputKeys<'a, Message>) -> Self {
        Element::new(value)
    }
}

/// What the wrapper remembers between events.
#[derive(Default)]
struct State {
    /// Whether the wrapped input had the focus after the last event.
    was_focused: bool,
    /// The text the app gave the input at the last rebuild.
    text: String,
    /// Whether the input got a key since the last rebuild, so a new text is
    /// its own edit and not a replacement from outside.
    input_edited: bool,
}

/// The state of the text input wrapped by the [`InputKeys`] of `tree`.
fn input_state_mut(
    tree: &mut Tree,
) -> &mut text_input::State<<Renderer as text::Renderer>::Paragraph> {
    tree.children[0]
        .state
        .downcast_mut::<text_input::State<<Renderer as text::Renderer>::Paragraph>>()
}

/// Whether the text input wrapped by the [`InputKeys`] of `tree` has the
/// focus.
fn input_is_focused(tree: &Tree) -> bool {
    tree.children[0]
        .state
        .downcast_ref::<text_input::State<<Renderer as text::Renderer>::Paragraph>>()
        .is_focused()
}

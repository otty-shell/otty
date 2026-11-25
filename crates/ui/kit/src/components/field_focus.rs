//! Tab and Shift+Tab between the fields of a form or a dialog.
//!
//! `iced` 0.14 moves no focus on Tab, and its `focus_next` operation walks
//! the whole window and stops at its end. [`FieldFocus`] walks only the
//! fields inside it and goes round: from the last field Tab goes to the
//! first one, and from the first one Shift+Tab goes to the last. The kit
//! wraps every [`Dialog`] and [`FormPage`] in it, so no form has Tab code of
//! its own.
//!
//! A field is a widget that takes the keyboard (a text input). A field that
//! uses Tab itself, such as a field whose suggestion row is lit or a
//! terminal, captures it first, and then nothing moves.
//!
//! [`Dialog`]: super::dialog::Dialog
//! [`FormPage`]: super::form_page::FormPage

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::operation::focusable::{self, Count, Focusable};
use iced::advanced::widget::{Id, Operation, Tree, Widget, operation};
use iced::advanced::{Clipboard, Shell, overlay, renderer};
use iced::keyboard::key::Named;
use iced::{
    Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector, keyboard,
    mouse,
};

/// Content whose fields Tab and Shift+Tab walk, round and round.
pub(super) struct FieldFocus<'a, Message> {
    content: Element<'a, Message>,
    is_modal: bool,
}

impl<'a, Message> FieldFocus<'a, Message> {
    /// The fields of a form among other widgets of the window: Tab walks
    /// them only from a field of the form, so a field outside it keeps its
    /// Tab.
    pub(super) fn form(content: impl Into<Element<'a, Message>>) -> Self {
        Self {
            content: content.into(),
            is_modal: false,
        }
    }

    /// The fields of a modal dialog: Tab always stays in the dialog. With
    /// no field of the dialog holding the keyboard, Tab gives it to the
    /// first field and Shift+Tab to the last one.
    pub(super) fn modal(content: impl Into<Element<'a, Message>>) -> Self {
        Self {
            content: content.into(),
            is_modal: true,
        }
    }
}

impl<Message> Widget<Message, Theme, Renderer> for FieldFocus<'_, Message> {
    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
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

        if shell.is_event_captured() {
            return;
        }
        let Some(step) = tab_step(event) else {
            return;
        };

        let mut count = focusable::count();
        self.content.as_widget_mut().operate(
            &mut tree.children[0],
            layout,
            renderer,
            &mut operation::black_box(&mut count),
        );
        let fields = match count.finish() {
            operation::Outcome::Some(fields) => fields,
            _ => Count::default(),
        };

        let Some(target) = next_field(fields, step, self.is_modal) else {
            if self.is_modal {
                shell.capture_event();
            }
            return;
        };

        self.content.as_widget_mut().operate(
            &mut tree.children[0],
            layout,
            renderer,
            &mut FocusField { target, current: 0 },
        );
        shell.capture_event();
        shell.request_redraw();
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

impl<'a, Message: 'a> From<FieldFocus<'a, Message>> for Element<'a, Message> {
    fn from(value: FieldFocus<'a, Message>) -> Self {
        Element::new(value)
    }
}

/// Gives the keyboard to the field at `target`, counted in the order the
/// fields are laid out, and takes it from every other field.
struct FocusField {
    target: usize,
    current: usize,
}

impl Operation for FocusField {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn focusable(
        &mut self,
        _id: Option<&Id>,
        _bounds: Rectangle,
        state: &mut dyn Focusable,
    ) {
        if self.current == self.target {
            state.focus();
        } else {
            state.unfocus();
        }

        self.current += 1;
    }
}

/// Which way a press of Tab moves the keyboard.
#[derive(Debug, Clone, Copy)]
enum Step {
    /// Tab: to the next field.
    Next,
    /// Shift+Tab: to the field before.
    Previous,
}

/// The step a press of Tab or Shift+Tab stands for; `None` for any other
/// event, and for Tab with Ctrl, Alt or the logo key held, which are not
/// moves between fields.
fn tab_step(event: &Event) -> Option<Step> {
    let Event::Keyboard(keyboard::Event::KeyPressed {
        key: keyboard::Key::Named(Named::Tab),
        modifiers,
        ..
    }) = event
    else {
        return None;
    };

    if modifiers.control() || modifiers.alt() || modifiers.logo() {
        return None;
    }

    if modifiers.shift() {
        Some(Step::Previous)
    } else {
        Some(Step::Next)
    }
}

/// The place of the field that gets the keyboard after `step`, among
/// `fields`: the next or the previous one, round from the end to the start.
/// With none of them holding the keyboard, a modal content starts from its
/// first field (or its last for Shift+Tab), and other content moves nothing.
fn next_field(fields: Count, step: Step, is_modal: bool) -> Option<usize> {
    if fields.total == 0 {
        return None;
    }

    match (fields.focused, step) {
        (Some(focused), Step::Next) => Some((focused + 1) % fields.total),
        (Some(focused), Step::Previous) => {
            Some((focused + fields.total - 1) % fields.total)
        },
        (None, Step::Next) if is_modal => Some(0),
        (None, Step::Previous) if is_modal => Some(fields.total - 1),
        _ => None,
    }
}

//! The title, the body and the actions of a dialog's card, top to bottom.
//!
//! `iced`'s column measures its children in order, so a body that scrolls
//! would take the whole height before the actions under it are measured,
//! and a body that fills would make every card as tall as the window.
//! [`DialogColumn`] measures the title and the actions first and gives the
//! body only the height they leave, so in a low window the body scrolls and
//! the title and the actions stay on the card.

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{Operation, Tree, Widget};
use iced::advanced::{Clipboard, Shell, overlay, renderer};
use iced::{
    Element, Event, Length, Point, Rectangle, Renderer, Size, Theme, Vector,
    mouse,
};

/// The parts of a dialog's card, `spacing` apart. Only the body gives way
/// when the card has less height than its parts want.
pub(super) struct DialogColumn<'a, Message> {
    parts: Vec<Element<'a, Message>>,
    body: Option<usize>,
    spacing: f32,
}

impl<'a, Message> DialogColumn<'a, Message> {
    /// A card with only its `title`.
    pub(super) fn new(title: Element<'a, Message>, spacing: f32) -> Self {
        Self {
            parts: vec![title],
            body: None,
            spacing,
        }
    }

    /// Puts the body under the title. It gets the height the title and the
    /// actions leave.
    pub(super) fn body(mut self, body: Element<'a, Message>) -> Self {
        self.body = Some(self.parts.len());
        self.parts.push(body);
        self
    }

    /// Puts the row of actions at the bottom.
    pub(super) fn actions(mut self, actions: Element<'a, Message>) -> Self {
        self.parts.push(actions);
        self
    }
}

impl<Message> Widget<Message, Theme, Renderer> for DialogColumn<'_, Message> {
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Shrink)
    }

    fn children(&self) -> Vec<Tree> {
        self.parts.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.parts);
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let width = limits.max().width;
        let unbounded =
            layout::Limits::new(Size::ZERO, Size::new(width, f32::INFINITY));
        let gaps = self.spacing * self.parts.len().saturating_sub(1) as f32;

        let mut nodes = vec![layout::Node::default(); self.parts.len()];
        let mut taken = gaps;
        for (index, (part, tree)) in
            self.parts.iter_mut().zip(&mut tree.children).enumerate()
        {
            if Some(index) == self.body {
                continue;
            }
            let node = part.as_widget_mut().layout(tree, renderer, &unbounded);
            taken += node.size().height;
            nodes[index] = node;
        }

        if let Some(index) = self.body {
            let left = (limits.max().height - taken).max(0.0);
            let body_limits =
                layout::Limits::new(Size::ZERO, Size::new(width, left));
            nodes[index] = self.parts[index].as_widget_mut().layout(
                &mut tree.children[index],
                renderer,
                &body_limits,
            );
        }

        let mut y = 0.0;
        for node in &mut nodes {
            node.move_to_mut(Point::new(0.0, y));
            y += node.size().height + self.spacing;
        }
        let height = (y - self.spacing).max(0.0);

        layout::Node::with_children(Size::new(width, height), nodes)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            for ((part, tree), layout) in self
                .parts
                .iter_mut()
                .zip(&mut tree.children)
                .zip(layout.children())
            {
                part.as_widget_mut()
                    .operate(tree, layout, renderer, operation);
            }
        });
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
        for ((part, tree), layout) in self
            .parts
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
        {
            part.as_widget_mut().update(
                tree, event, layout, cursor, renderer, clipboard, shell,
                viewport,
            );
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.parts
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
            .map(|((part, tree), layout)| {
                part.as_widget()
                    .mouse_interaction(tree, layout, cursor, viewport, renderer)
            })
            .max()
            .unwrap_or_default()
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
        for ((part, tree), layout) in
            self.parts.iter().zip(&tree.children).zip(layout.children())
        {
            part.as_widget()
                .draw(tree, renderer, theme, style, layout, cursor, viewport);
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        overlay::from_children(
            &mut self.parts,
            tree,
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message: 'a> From<DialogColumn<'a, Message>> for Element<'a, Message> {
    fn from(value: DialogColumn<'a, Message>) -> Self {
        Element::new(value)
    }
}

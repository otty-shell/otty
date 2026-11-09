//! Lights an icon up while the pointer is over the whole button around it,
//! and a row up while the pointer is over it.
//!
//! `iced` gives an `svg` a hovered state only while the pointer is over the
//! svg itself, and does not pass the state of a `button` down to its content.
//! A 16px icon in a 28px button would therefore keep its idle color while the
//! pointer is on the button's edge. [`IconHover`] closes the gap: it watches
//! the control (the button, or the container that acts as one) and tells the
//! icons inside whether the pointer is over it.
//!
//! A `container` has no hovered state at all, so a row that takes no presses
//! of its own, and so is no `button`, uses the same watcher for its
//! background.

use std::cell::Cell;
use std::rc::Rc;

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{Operation, Tree, Widget};
use iced::advanced::{Clipboard, Shell, overlay, renderer};
use iced::widget::{container, svg};
use iced::{
    Color, Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector,
    mouse,
};

/// Whether the pointer is over a control, shared by the control's watcher and
/// the icon styles inside it.
#[derive(Clone, Default)]
pub(super) struct IconHover {
    is_hovered: Rc<Cell<bool>>,
}

impl IconHover {
    /// Creates a state with the pointer away.
    pub(super) fn new() -> Self {
        Self::default()
    }

    /// Returns the svg style of an icon inside the watched control: `idle`
    /// while the pointer is away, `hover` while it is over the control.
    pub(super) fn icon_style(
        &self,
        idle: Color,
        hover: Color,
    ) -> impl Fn(&Theme, svg::Status) -> svg::Style + 'static {
        let is_hovered = Rc::clone(&self.is_hovered);

        move |_, _| svg::Style {
            color: Some(if is_hovered.get() { hover } else { idle }),
        }
    }

    /// Returns the style of the watched container itself: `idle` while the
    /// pointer is away, `hover` while it is over the container.
    pub(super) fn container_style(
        &self,
        idle: container::Style,
        hover: container::Style,
    ) -> impl Fn(&Theme) -> container::Style + 'static {
        let is_hovered = Rc::clone(&self.is_hovered);

        move |_| if is_hovered.get() { hover } else { idle }
    }

    /// Wraps `control` so that it reports the pointer to this state. The
    /// wrapper has the size and look of `control`; it draws nothing itself.
    pub(super) fn watch<'a, Message: 'a>(
        &self,
        control: impl Into<Element<'a, Message>>,
    ) -> Element<'a, Message> {
        Element::new(Watcher {
            control: control.into(),
            is_hovered: Rc::clone(&self.is_hovered),
        })
    }
}

/// Passes everything to `control` and records whether the pointer is over
/// its bounds, before the control updates and before it draws.
struct Watcher<'a, Message> {
    control: Element<'a, Message>,
    is_hovered: Rc<Cell<bool>>,
}

impl<Message> Widget<Message, Theme, Renderer> for Watcher<'_, Message> {
    fn size(&self) -> Size<Length> {
        self.control.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.control.as_widget().size_hint()
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.control)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[self.control.as_widget()]);
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.control.as_widget_mut().layout(
            &mut tree.children[0],
            renderer,
            limits,
        )
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
        let is_over = cursor.is_over(layout.bounds());
        if self.is_hovered.replace(is_over) != is_over {
            shell.request_redraw();
        }

        self.control.as_widget_mut().update(
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
        self.is_hovered.set(cursor.is_over(layout.bounds()));

        self.control.as_widget().draw(
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
        self.control.as_widget().mouse_interaction(
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
        self.control.as_widget_mut().operate(
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
        self.control.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

#[cfg(test)]
pub(super) mod check {
    use iced::widget::container;
    use iced::{Element, Event, Point, Rectangle, mouse};

    use crate::test_support::TempDir;

    /// Where the tested control is placed in the test window.
    const ORIGIN: f32 = 40.0;

    /// Draws `view` with no pointer, with the pointer on the middle of
    /// `control` (a rectangle inside `view`) and with the pointer one pixel
    /// inside its top-left corner, away from the icon. The corner picture
    /// must equal the middle one (the icon lights up over the whole control)
    /// and both must differ from the idle one.
    pub(in crate::components) fn assert_icon_lights_up(
        name: &str,
        view: impl Fn() -> Element<'static, ()>,
        control: Rectangle,
    ) {
        let folder = TempDir::new("icon-hover");
        let reference = folder.path().join(name);
        let middle = Point::new(
            ORIGIN + control.x + control.width / 2.0,
            ORIGIN + control.y + control.height / 2.0,
        );
        let corner =
            Point::new(ORIGIN + control.x + 1.0, ORIGIN + control.y + 1.0);

        // Saves the picture of the pointer on the icon.
        assert!(draws(&view, Some(middle), &reference), "{name}: save");

        assert!(
            draws(&view, Some(corner), &reference),
            "{name}: the icon must light up with the pointer on the corner"
        );
        assert!(
            !draws(&view, None, &reference),
            "{name}: the idle picture must differ from the hovered one"
        );
    }

    fn draws(
        view: &impl Fn() -> Element<'static, ()>,
        pointer: Option<Point>,
        reference: &std::path::Path,
    ) -> bool {
        let mut ui = iced_test::simulator(
            container(view()).padding(ORIGIN).width(240).height(160),
        );
        if let Some(position) = pointer {
            ui.point_at(position);
            let _ = ui.simulate([Event::Mouse(mouse::Event::CursorMoved {
                position,
            })]);
        }

        ui.snapshot(&iced::Theme::Dark)
            .expect("control should draw")
            .matches_hash(reference)
            .expect("reference should be readable")
    }
}

//! The "⋯" button at the end of a [`Toolbar`] and the menu it opens.
//!
//! The toolbar decides on each layout which buttons do not fit, so only the
//! kit knows which items the menu holds and where the button is. The
//! button therefore keeps its own open state and draws the menu as an
//! overlay under itself, as a pick list does, instead of asking the app to
//! place a menu layer.
//!
//! [`Toolbar`]: super::toolbar::Toolbar

use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer::{self, Renderer as _};
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell, overlay};
use iced::widget::{container, svg, tooltip};
use iced::{
    Border, Element, Event, Length, Point, Rectangle, Renderer, Size, Theme,
    Vector, keyboard, mouse,
};

use super::{glyph, icon_button};
use crate::palette::UiPalette;
use crate::{radius, size, spacing};

/// Edge of the dots inside the button, as in a medium `IconButton`.
const ICON_SIZE: f32 = 16.0;
/// Gap between the button and the menu under it.
const MENU_GAP: f32 = spacing::SPACE_1;

/// A medium "⋯" icon button that opens `menu` under itself, aligned to its
/// right edge. The menu closes when one of its items is pressed, on a
/// press outside it (the "⋯" button included) and on Escape.
pub(super) struct OverflowMenu<'a, Message> {
    button: Element<'a, Message>,
    menu: Element<'a, Message>,
    palette: UiPalette,
}

impl<'a, Message: 'a> OverflowMenu<'a, Message> {
    /// Creates the button with the tooltip `label` over `menu`.
    pub(super) fn new(
        label: String,
        menu: impl Into<Element<'a, Message>>,
        palette: &UiPalette,
    ) -> Self {
        let idle = palette.text_muted();
        let hover = palette.text_strong();

        let dots = svg(glyph::dots()).width(ICON_SIZE).height(ICON_SIZE).style(
            move |_, status| svg::Style {
                color: Some(match status {
                    svg::Status::Hovered => hover,
                    _ => idle,
                }),
            },
        );
        let button = tooltip(
            container(dots).center(size::CONTROL_MD),
            icon_button::tooltip_hint(label, palette),
            tooltip::Position::Bottom,
        )
        .gap(spacing::SPACE_1);

        Self {
            button: button.into(),
            menu: menu.into(),
            palette: palette.clone(),
        }
    }
}

impl<Message> Widget<Message, Theme, Renderer> for OverflowMenu<'_, Message> {
    fn size(&self) -> Size<Length> {
        self.button.as_widget().size()
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.button), Tree::new(&self.menu)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[self.button.as_widget(), self.menu.as_widget()]);
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.button.as_widget_mut().layout(
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
        if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) =
            event
            && cursor.is_over(layout.bounds())
        {
            let state = tree.state.downcast_mut::<State>();
            state.is_open = !state.is_open;

            shell.capture_event();
            shell.invalidate_layout();
            shell.request_redraw();

            return;
        }

        self.button.as_widget_mut().update(
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
        let state = tree.state.downcast_ref::<State>();
        let bounds = layout.bounds();

        let background = if state.is_open {
            Some(self.palette.surface_hover())
        } else if cursor.is_over(bounds) {
            Some(self.palette.surface_raised())
        } else {
            None
        };
        if let Some(background) = background {
            renderer.fill_quad(
                renderer::Quad {
                    bounds,
                    border: Border {
                        radius: radius::RADIUS_SM.into(),
                        ..Border::default()
                    },
                    ..renderer::Quad::default()
                },
                background,
            );
        }

        self.button.as_widget().draw(
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
        _tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        if cursor.is_over(layout.bounds()) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::None
        }
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.button.as_widget_mut().operate(
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
        let Tree {
            state, children, ..
        } = tree;
        let state = state.downcast_mut::<State>();
        let [button_tree, menu_tree] = children.as_mut_slice() else {
            return None;
        };

        if !state.is_open {
            return self.button.as_widget_mut().overlay(
                button_tree,
                layout,
                renderer,
                viewport,
                translation,
            );
        }

        Some(overlay::Element::new(Box::new(MenuOverlay {
            is_open: &mut state.is_open,
            menu: &mut self.menu,
            tree: menu_tree,
            anchor: layout.bounds() + translation,
        })))
    }
}

impl<'a, Message: 'a> From<OverflowMenu<'a, Message>> for Element<'a, Message> {
    fn from(value: OverflowMenu<'a, Message>) -> Self {
        Element::new(value)
    }
}

/// Whether the menu of an [`OverflowMenu`] is open.
#[derive(Default)]
struct State {
    is_open: bool,
}

/// The open menu, drawn over everything under the "⋯" button at `anchor`.
struct MenuOverlay<'a, 'b, Message> {
    is_open: &'a mut bool,
    menu: &'a mut Element<'b, Message>,
    tree: &'a mut Tree,
    anchor: Rectangle,
}

impl<Message> MenuOverlay<'_, '_, Message> {
    /// Closes the menu and asks for a new layout, so the overlay goes away.
    fn close(&mut self, shell: &mut Shell<'_, Message>) {
        *self.is_open = false;

        shell.invalidate_layout();
        shell.request_redraw();
    }
}

impl<Message> overlay::Overlay<Message, Theme, Renderer>
    for MenuOverlay<'_, '_, Message>
{
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        let node = self.menu.as_widget_mut().layout(
            self.tree,
            renderer,
            &layout::Limits::new(Size::ZERO, bounds),
        );

        let position = menu_position(self.anchor, node.size(), bounds);

        node.move_to(position)
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        self.menu.as_widget().draw(
            self.tree,
            renderer,
            theme,
            style,
            layout,
            cursor,
            &layout.bounds(),
        );
    }

    fn operate(
        &mut self,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.menu
            .as_widget_mut()
            .operate(self.tree, layout, renderer, operation);
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        let bounds = layout.bounds();

        self.menu.as_widget_mut().update(
            self.tree, event, layout, cursor, renderer, clipboard, shell,
            &bounds,
        );

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                if !cursor.is_over(bounds) =>
            {
                shell.capture_event();
                self.close(shell);
            },
            // A captured release over the menu is a pressed item.
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                if cursor.is_over(bounds) && shell.is_event_captured() =>
            {
                self.close(shell);
            },
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(keyboard::key::Named::Escape),
                ..
            }) => {
                shell.capture_event();
                self.close(shell);
            },
            _ => {},
        }
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.menu.as_widget().mouse_interaction(
            self.tree,
            layout,
            cursor,
            &layout.bounds(),
            renderer,
        )
    }
}

/// Returns the top-left corner of a menu of `menu` size for a button at
/// `anchor` in a window of `bounds` size: under the button with its right
/// edge on the button's right edge, or over the button when there is no
/// room under it, and never past the left or top edge.
fn menu_position(anchor: Rectangle, menu: Size, bounds: Size) -> Point {
    let x = (anchor.x + anchor.width - menu.width).max(0.0);

    let below = anchor.y + anchor.height + MENU_GAP;
    let y = if below + menu.height <= bounds.height {
        below
    } else {
        (anchor.y - MENU_GAP - menu.height).max(0.0)
    };

    Point::new(x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    const WINDOW: Size = Size::new(800.0, 600.0);
    const MENU: Size = Size::new(240.0, 100.0);

    #[test]
    fn given_room_under_the_button_then_menu_opens_under_it_right_aligned() {
        let anchor =
            Rectangle::new(Point::new(500.0, 40.0), Size::new(28.0, 28.0));

        let position = menu_position(anchor, MENU, WINDOW);

        assert_eq!(position, Point::new(528.0 - 240.0, 68.0 + MENU_GAP));
    }

    #[test]
    fn given_no_room_under_the_button_then_menu_opens_over_it() {
        let anchor =
            Rectangle::new(Point::new(500.0, 560.0), Size::new(28.0, 28.0));

        let position = menu_position(anchor, MENU, WINDOW);

        assert_eq!(position, Point::new(288.0, 560.0 - MENU_GAP - 100.0));
    }

    #[test]
    fn given_button_near_the_left_edge_then_menu_stays_in_the_window() {
        let anchor =
            Rectangle::new(Point::new(10.0, 40.0), Size::new(28.0, 28.0));

        let position = menu_position(anchor, MENU, WINDOW);

        assert_eq!(position.x, 0.0);
    }
}

//! The row of mode tabs inside a [`ModeSwitcher`]: the tabs scroll
//! sideways when they do not fit, and the "⋯" More button stays at the
//! right end, over them.
//!
//! The tabs lie in a horizontal `scrollable` with no bar: a 4px bar would
//! lie over the tabs of the 32px dock and hide its right end under More.
//! A soft fade at a cut edge shows that more tabs lie past it instead. The
//! row ends in a free space as wide as More, so the last tab can come out
//! from under it. Any wheel or trackpad move over the dock scrolls the tabs
//! sideways, and the active tab is scrolled into view when it changes.
//!
//! [`ModeSwitcher`]: super::mode_switcher::ModeSwitcher

use std::f32::consts::FRAC_PI_2;

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::operation::Scrollable;
use iced::advanced::widget::operation::scrollable::AbsoluteOffset;
use iced::advanced::widget::{Id, Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Renderer as _, Shell, overlay, renderer};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{Space, row, scrollable};
use iced::{
    Background, Color, Element, Event, Gradient, Length, Point, Rectangle,
    Renderer, Size, Theme, Vector, gradient, mouse,
};

use super::floating_list::offset_showing;
use crate::{size, spacing};

/// Gap between tabs (`gap: 2px`), and between the last tab and More.
pub(super) const TAB_GAP: f32 = 2.0;
/// Width of the fade at a cut edge of the tabs.
const FADE_WIDTH: f32 = spacing::SPACE_4;
/// From one tab to the next; one notch of the wheel moves the tabs by it.
const TAB_STRIDE: f32 = size::MODE_TAB_WIDTH + TAB_GAP;

/// The tabs, which scroll sideways, and More over their right end.
pub(super) struct ModeDock<'a, Message> {
    /// The scrolling tabs, then More if there is one.
    children: Vec<Element<'a, Message>>,
    active: Option<usize>,
    background: Color,
}

impl<'a, Message: 'a> ModeDock<'a, Message> {
    /// Lays `tabs` out in a row that scrolls sideways, with `more` at its
    /// right end. The tab at `active` is kept in view when it changes;
    /// `background` is the color of the dock, drawn behind More and in the
    /// fades.
    pub(super) fn new(
        tabs: Vec<Element<'a, Message>>,
        more: Option<Element<'a, Message>>,
        active: Option<usize>,
        background: Color,
    ) -> Self {
        let mut strip = row(tabs).spacing(TAB_GAP);
        if more.is_some() {
            strip = strip.push(Space::new().width(size::MODE_TAB_WIDTH));
        }
        let tabs = scrollable(strip)
            .direction(Direction::Horizontal(Scrollbar::hidden()));

        let mut children = vec![Element::from(tabs)];
        children.extend(more);

        Self {
            children,
            active,
            background,
        }
    }

    /// Where More covers the tabs: More and the gap before it, or nothing
    /// without More.
    fn cover(&self, layout: Layout<'_>) -> Option<Rectangle> {
        let more = layout.children().nth(1)?.bounds();

        Some(Rectangle {
            x: more.x - TAB_GAP,
            width: more.width + TAB_GAP,
            ..more
        })
    }

    /// The pointer as the tabs see it: none while it is over More.
    fn tabs_cursor(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) -> mouse::Cursor {
        match self.cover(layout) {
            Some(cover) if cursor.is_over(cover) => cursor.levitate(),
            _ => cursor,
        }
    }

    /// Scrolls the tabs just enough to show the active one, when it is not
    /// the one shown last. The part of the row under More does not count as
    /// shown.
    fn keep_active_in_view(
        &mut self,
        state: &mut State,
        tree: &mut Tree,
        node: &layout::Node,
        renderer: &Renderer,
    ) {
        if state.shown_active == self.active {
            return;
        }
        state.shown_active = self.active;

        let Some(active) = self.active else {
            return;
        };

        let left = active as f32 * TAB_STRIDE;
        let mut show = ShowTab {
            left,
            right: left + size::MODE_TAB_WIDTH,
            covered: self.covered_width(),
        };
        self.children[0].as_widget_mut().operate(
            tree,
            Layout::new(node),
            renderer,
            &mut show,
        );
    }

    /// How much of the right end of the tabs More and its gap cover.
    fn covered_width(&self) -> f32 {
        if self.children.len() > 1 {
            TAB_STRIDE
        } else {
            0.0
        }
    }

    /// Reads how far the tabs are scrolled and how far they can go.
    fn measure(
        &mut self,
        state: &mut State,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
    ) {
        let mut probe = ScrollProbe::default();
        self.children[0]
            .as_widget_mut()
            .operate(tree, layout, renderer, &mut probe);

        state.offset = probe.offset;
        state.overflow = probe.overflow;
    }

    /// Draws a fade into the dock color at each cut edge of the tabs.
    fn draw_fades(
        &self,
        state: &State,
        renderer: &mut Renderer,
        tabs: Rectangle,
    ) {
        let edges = cut_edges(state.offset, state.overflow);
        let clear = Color {
            a: 0.0,
            ..self.background
        };

        if edges.left {
            let bounds = Rectangle {
                width: FADE_WIDTH,
                ..tabs
            };
            fill_fade(renderer, bounds, self.background, clear);
        }
        if edges.right {
            let right = tabs.x + tabs.width - self.covered_width();
            let bounds = Rectangle {
                x: right - FADE_WIDTH,
                width: FADE_WIDTH,
                ..tabs
            };
            fill_fade(renderer, bounds, clear, self.background);
        }
    }
}

impl<'a, Message: 'a> Widget<Message, Theme, Renderer>
    for ModeDock<'a, Message>
{
    fn size(&self) -> Size<Length> {
        Size::new(Length::Shrink, Length::Shrink)
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.children);
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let Tree {
            state, children, ..
        } = tree;
        let state = state.downcast_mut::<State>();

        let tabs = self.children[0].as_widget_mut().layout(
            &mut children[0],
            renderer,
            limits,
        );
        let size = tabs.size();
        self.keep_active_in_view(state, &mut children[0], &tabs, renderer);
        self.measure(state, &mut children[0], Layout::new(&tabs), renderer);

        let mut nodes = vec![tabs];
        if let Some(more) = self.children.get_mut(1) {
            let node = more.as_widget_mut().layout(
                &mut children[1],
                renderer,
                &layout::Limits::new(
                    Size::ZERO,
                    Size::new(f32::INFINITY, f32::INFINITY),
                ),
            );
            let x = (size.width - node.size().width).max(0.0);
            nodes.push(node.move_to(Point::new(x, 0.0)));
        }

        layout::Node::with_children(size, nodes)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.traverse(&mut |operation| {
            for ((child, tree), layout) in self
                .children
                .iter_mut()
                .zip(&mut tree.children)
                .zip(layout.children())
            {
                child
                    .as_widget_mut()
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
        let Tree {
            state, children, ..
        } = tree;
        let state = state.downcast_mut::<State>();
        let mut layouts = layout.children();
        let Some(tabs_layout) = layouts.next() else {
            return;
        };

        if let Event::Mouse(mouse::Event::WheelScrolled { delta }) = event
            && state.overflow > 0.0
            && cursor.is_over(layout.bounds())
        {
            let mut scroll = ScrollBy {
                x: sideways_scroll(*delta),
            };
            self.children[0].as_widget_mut().operate(
                &mut children[0],
                tabs_layout,
                renderer,
                &mut scroll,
            );
            self.measure(state, &mut children[0], tabs_layout, renderer);

            shell.capture_event();
            shell.request_redraw();
            return;
        }

        if let (Some(more), Some(more_layout)) =
            (self.children.get_mut(1), layouts.next())
        {
            more.as_widget_mut().update(
                &mut children[1],
                event,
                more_layout,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );
            if shell.is_event_captured() {
                return;
            }
        }

        let tabs_cursor = self.tabs_cursor(layout, cursor);
        self.children[0].as_widget_mut().update(
            &mut children[0],
            event,
            tabs_layout,
            tabs_cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
        self.measure(state, &mut children[0], tabs_layout, renderer);
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
        let Some(visible) = layout.bounds().intersection(viewport) else {
            return;
        };
        let state = tree.state.downcast_ref::<State>();
        let mut layouts = layout.children();
        let Some(tabs_layout) = layouts.next() else {
            return;
        };

        self.children[0].as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            tabs_layout,
            self.tabs_cursor(layout, cursor),
            viewport,
        );

        // The tab icons are drawn after every quad of their layer, so the
        // fades and More go on a layer of their own over them.
        renderer.with_layer(visible, |renderer| {
            self.draw_fades(state, renderer, tabs_layout.bounds());

            let (Some(more), Some(more_layout), Some(cover)) =
                (self.children.get(1), layouts.next(), self.cover(layout))
            else {
                return;
            };
            renderer.fill_quad(
                renderer::Quad {
                    bounds: cover,
                    ..renderer::Quad::default()
                },
                self.background,
            );
            more.as_widget().draw(
                &tree.children[1],
                renderer,
                theme,
                style,
                more_layout,
                cursor,
                viewport,
            );
        });
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        let mut layouts = layout.children();
        let Some(tabs_layout) = layouts.next() else {
            return mouse::Interaction::None;
        };

        if let (Some(more), Some(more_layout), Some(cover)) =
            (self.children.get(1), layouts.next(), self.cover(layout))
            && cursor.is_over(cover)
        {
            return more.as_widget().mouse_interaction(
                &tree.children[1],
                more_layout,
                cursor,
                viewport,
                renderer,
            );
        }

        self.children[0].as_widget().mouse_interaction(
            &tree.children[0],
            tabs_layout,
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
        overlay::from_children(
            &mut self.children,
            tree,
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message: 'a> From<ModeDock<'a, Message>> for Element<'a, Message> {
    fn from(value: ModeDock<'a, Message>) -> Self {
        Element::new(value)
    }
}

/// What a [`ModeDock`] remembers between frames.
#[derive(Default)]
struct State {
    /// The active tab the dock last scrolled to. The dock scrolls only when
    /// this changes, so the wheel can move the tabs freely.
    shown_active: Option<usize>,
    /// How far the tabs are scrolled, in pixels.
    offset: f32,
    /// How far the tabs can scroll at most; zero when they fit.
    overflow: f32,
}

/// The cut edges of the tabs, which a fade covers.
#[derive(Debug, PartialEq, Eq)]
struct CutEdges {
    /// Tabs lie past the left edge.
    left: bool,
    /// Tabs lie under More or past the right edge.
    right: bool,
}

/// Scrolls the tabs just enough to show the tab spanning `left..right` of
/// the row; the `covered` right end of the view does not show it.
struct ShowTab {
    left: f32,
    right: f32,
    covered: f32,
}

impl Operation for ShowTab {
    fn traverse(&mut self, _operate: &mut dyn FnMut(&mut dyn Operation)) {}

    fn scrollable(
        &mut self,
        _id: Option<&Id>,
        bounds: Rectangle,
        _content_bounds: Rectangle,
        translation: Vector,
        state: &mut dyn Scrollable,
    ) {
        let shown = (bounds.width - self.covered).max(0.0);
        let offset =
            offset_showing(self.left, self.right, translation.x, shown);

        if let Some(x) = offset {
            state.scroll_to(AbsoluteOffset {
                x: Some(x),
                y: None,
            });
        }
    }
}

/// Scrolls the tabs sideways by `x` pixels.
struct ScrollBy {
    x: f32,
}

impl Operation for ScrollBy {
    fn traverse(&mut self, _operate: &mut dyn FnMut(&mut dyn Operation)) {}

    fn scrollable(
        &mut self,
        _id: Option<&Id>,
        bounds: Rectangle,
        content_bounds: Rectangle,
        _translation: Vector,
        state: &mut dyn Scrollable,
    ) {
        state.scroll_by(
            AbsoluteOffset { x: self.x, y: 0.0 },
            bounds,
            content_bounds,
        );
    }
}

/// Reads how far the tabs are scrolled and how far they can go.
#[derive(Default)]
struct ScrollProbe {
    offset: f32,
    overflow: f32,
}

impl Operation for ScrollProbe {
    fn traverse(&mut self, _operate: &mut dyn FnMut(&mut dyn Operation)) {}

    fn scrollable(
        &mut self,
        _id: Option<&Id>,
        bounds: Rectangle,
        content_bounds: Rectangle,
        translation: Vector,
        _state: &mut dyn Scrollable,
    ) {
        self.offset = translation.x;
        self.overflow = (content_bounds.width - bounds.width).max(0.0);
    }
}

/// How far a wheel or trackpad move scrolls the tabs sideways, in pixels:
/// along its larger part, so a plain wheel scrolls them too. A notch of the
/// wheel down moves the tabs by one tab to the left.
fn sideways_scroll(delta: mouse::ScrollDelta) -> f32 {
    let (x, y, pixels) = match delta {
        mouse::ScrollDelta::Lines { x, y } => (x, y, TAB_STRIDE),
        mouse::ScrollDelta::Pixels { x, y } => (x, y, 1.0),
    };
    let along = if x.abs() > y.abs() { x } else { y };

    -along * pixels
}

/// Which edges of the tabs, scrolled to `offset` of at most `overflow`, cut
/// tabs off.
fn cut_edges(offset: f32, overflow: f32) -> CutEdges {
    CutEdges {
        left: offset > 0.0,
        right: offset < overflow,
    }
}

/// Fills `bounds` with a fade from `left` to `right`.
fn fill_fade(
    renderer: &mut Renderer,
    bounds: Rectangle,
    left: Color,
    right: Color,
) {
    let fade = gradient::Linear::new(FRAC_PI_2)
        .add_stop(0.0, left)
        .add_stop(1.0, right);

    renderer.fill_quad(
        renderer::Quad {
            bounds,
            ..renderer::Quad::default()
        },
        Background::Gradient(Gradient::Linear(fade)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_notch_of_the_wheel_moves_the_tabs_by_one_tab() {
        let down = mouse::ScrollDelta::Lines { x: 0.0, y: -1.0 };
        let up = mouse::ScrollDelta::Lines { x: 0.0, y: 1.0 };

        assert_eq!(sideways_scroll(down), TAB_STRIDE);
        assert_eq!(sideways_scroll(up), -TAB_STRIDE);
    }

    #[test]
    fn a_trackpad_moves_the_tabs_along_its_larger_part() {
        let sideways = mouse::ScrollDelta::Pixels { x: -10.0, y: 2.0 };
        let down = mouse::ScrollDelta::Pixels { x: 1.0, y: -7.0 };

        assert_eq!(sideways_scroll(sideways), 10.0);
        assert_eq!(sideways_scroll(down), 7.0);
    }

    #[test]
    fn tabs_that_fit_have_no_cut_edge() {
        assert_eq!(
            cut_edges(0.0, 0.0),
            CutEdges {
                left: false,
                right: false
            }
        );
    }

    #[test]
    fn scrolled_tabs_are_cut_on_each_side_with_tabs_past_it() {
        assert_eq!(
            cut_edges(0.0, 40.0),
            CutEdges {
                left: false,
                right: true
            }
        );
        assert_eq!(
            cut_edges(20.0, 40.0),
            CutEdges {
                left: true,
                right: true
            }
        );
        assert_eq!(
            cut_edges(40.0, 40.0),
            CutEdges {
                left: true,
                right: false
            }
        );
    }
}

//! `Spinner`: a ring that turns while something loads.
//!
//! The spinner animates on its own: on every frame it turns the icon a bit
//! further and asks for the next frame, so the app needs no timer or
//! subscription to keep it moving. It turns only while it is on screen.

use std::f32::consts::TAU;
use std::time::{Duration, Instant};

use iced::advanced::layout::{self, Layout};
use iced::advanced::svg::{self as advanced_svg, Renderer as _};
use iced::advanced::widget::{Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell, renderer};
use iced::widget::svg;
use iced::{
    Color, Element, Event, Length, Radians, Rectangle, Renderer, Size, Theme,
    mouse, window,
};

use crate::palette::UiPalette;

/// Side of the spinner when the caller sets no size.
const DEFAULT_SIZE: f32 = 32.0;
/// How long one full turn takes.
const TURN: Duration = Duration::from_millis(900);

/// A turning ring drawn from the icon the caller passes, tinted with the
/// muted text color.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use otty_ui_kit::components::spinner::Spinner;
/// use otty_ui_kit::palette::UiPalette;
///
/// let palette = UiPalette::otty_dark();
/// let ring = iced::widget::svg::Handle::from_memory(&b"<svg/>"[..]);
/// let loading: Element<'_, ()> =
///     Spinner::new(ring, &palette).size(24.0).into();
/// ```
pub struct Spinner {
    icon: svg::Handle,
    color: Color,
    size: f32,
}

impl Spinner {
    /// Creates a spinner that turns `icon`, a ring with a gap.
    pub fn new(icon: svg::Handle, palette: &UiPalette) -> Self {
        Self {
            icon,
            color: palette.text_muted(),
            size: DEFAULT_SIZE,
        }
    }

    /// Sets the side of the square the spinner takes.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
}

impl<Message> Widget<Message, Theme, Renderer> for Spinner {
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fixed(self.size), Length::Fixed(self.size))
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::Node::new(limits.resolve(
            Length::Fixed(self.size),
            Length::Fixed(self.size),
            Size::new(self.size, self.size),
        ))
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let Event::Window(window::Event::RedrawRequested(now)) = event else {
            return;
        };

        let state = tree.state.downcast_mut::<State>();
        let started = *state.started.get_or_insert(*now);
        state.angle = angle_at(now.saturating_duration_since(started)).0;

        shell.request_redraw();
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State>();
        let icon = advanced_svg::Svg {
            handle: self.icon.clone(),
            color: Some(self.color),
            rotation: Radians(state.angle),
            opacity: 1.0,
        };

        renderer.draw_svg(icon, layout.bounds(), *viewport);
    }
}

impl<'a, Message: 'a> From<Spinner> for Element<'a, Message> {
    fn from(spinner: Spinner) -> Self {
        Element::new(spinner)
    }
}

/// When the spinner first drew and how far it has turned since, in
/// radians.
#[derive(Debug, Default)]
struct State {
    started: Option<Instant>,
    angle: f32,
}

/// How far the spinner has turned `elapsed` after it first drew; it starts
/// over after each full turn.
fn angle_at(elapsed: Duration) -> Radians {
    let turn = TURN.as_secs_f32();
    let into_turn = elapsed.as_secs_f32() % turn;

    Radians(into_turn / turn * TAU)
}

#[cfg(test)]
mod tests {
    use std::f32::consts::TAU;
    use std::time::Duration;

    use super::{TURN, angle_at};

    fn assert_near(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 1e-4,
            "{actual} is not {expected}"
        );
    }

    #[test]
    fn the_spinner_starts_unturned() {
        assert_near(angle_at(Duration::ZERO).0, 0.0);
    }

    #[test]
    fn a_quarter_of_the_turn_time_turns_it_a_quarter() {
        assert_near(angle_at(TURN / 4).0, TAU / 4.0);
    }

    #[test]
    fn after_a_full_turn_it_starts_over() {
        let later = TURN * 3 + TURN / 2;

        assert_near(angle_at(later).0, angle_at(TURN / 2).0);
    }
}

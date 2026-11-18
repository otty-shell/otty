//! An app that runs for a test over many frames.

use iced::advanced::clipboard;
use iced::advanced::renderer::Headless;
use iced::advanced::widget::{Operation, operation};
use iced::keyboard::key::Named;
use iced::{Element, Event, Point, Renderer, Size, Theme, mouse};
use iced_test::runtime::UserInterface;
use iced_test::runtime::user_interface::Cache;
use iced_test::{Selector, simulator};

/// The window of a [`LiveApp`].
const APP_WINDOW: Size = Size::new(400.0, 400.0);

/// An app that runs for a test: its state lives in the test, its view is
/// built again for each batch of events with the widget state iced keeps
/// between two frames, and each message it sends goes to its update. So a
/// test sees what a field does over many frames, as in the running app:
/// the focus, the cursor, a list that comes and goes.
pub(crate) struct LiveApp<State, Message> {
    state: State,
    view: fn(&State) -> Element<'_, Message>,
    update: fn(&mut State, Message),
    window: Size,
    cursor: mouse::Cursor,
    renderer: Renderer,
    cache: Option<Cache>,
}

impl<State, Message> LiveApp<State, Message> {
    pub(crate) fn state(&self) -> &State {
        &self.state
    }

    pub(crate) fn state_mut(&mut self) -> &mut State {
        &mut self.state
    }

    /// Starts the app with `state` in a 400 by 400 window, the pointer
    /// over its top left part.
    pub(crate) fn new(
        state: State,
        view: fn(&State) -> Element<'_, Message>,
        update: fn(&mut State, Message),
    ) -> Self {
        let renderer = iced::futures::executor::block_on(Renderer::new(
            iced::Font::with_name("Fira Sans"),
            iced::Pixels(16.0),
            None,
        ))
        .expect("a headless renderer");

        Self {
            state,
            view,
            update,
            window: APP_WINDOW,
            cursor: mouse::Cursor::Available(Point::new(100.0, 30.0)),
            renderer,
            cache: None,
        }
    }

    /// The same app in a window of `window` size.
    pub(crate) fn window(mut self, window: Size) -> Self {
        self.window = window;
        self
    }

    /// Moves the pointer to `point` for the next events.
    pub(crate) fn point_at(&mut self, point: Point) {
        self.cursor = mouse::Cursor::Available(point);
    }

    /// Gives `events` to the view built again and hands each message it
    /// sends to the update.
    pub(crate) fn send(&mut self, events: impl IntoIterator<Item = Event>) {
        let events: Vec<Event> = events.into_iter().collect();
        let cursor = self.cursor;
        let mut messages = Vec::new();

        self.with_view(|ui, renderer| {
            let _ = ui.update(
                &events,
                cursor,
                renderer,
                &mut clipboard::Null,
                &mut messages,
            );
        });

        for message in messages {
            (self.update)(&mut self.state, message);
        }
    }

    /// A press and a release of the left button where the pointer is.
    pub(crate) fn click(&mut self) {
        self.send(simulator::click());
    }

    pub(crate) fn type_text(&mut self, text: &str) {
        self.send(simulator::typewrite(text));
    }

    pub(crate) fn tap(&mut self, key: Named) {
        self.send(simulator::tap_key(key, None));
    }

    /// Finds what `selector` looks for in the view as it is now, such as a
    /// text and where it is drawn.
    pub(crate) fn find<S>(&mut self, selector: S) -> Option<S::Output>
    where
        S: Selector + Send,
        S::Output: Clone + Send,
    {
        let mut find = selector.find();

        self.with_view(|ui, renderer| {
            ui.operate(renderer, &mut operation::black_box(&mut find));
        });

        match find.finish() {
            operation::Outcome::Some(output) => output,
            _ => None,
        }
    }

    /// Builds the view again with the widget state of the last frame, hands
    /// it to `act` with the renderer and keeps the widget state for the
    /// next frame.
    fn with_view(
        &mut self,
        act: impl FnOnce(
            &mut UserInterface<'_, Message, Theme, Renderer>,
            &mut Renderer,
        ),
    ) {
        let mut ui = UserInterface::build(
            (self.view)(&self.state),
            self.window,
            self.cache.take().unwrap_or_default(),
            &mut self.renderer,
        );

        act(&mut ui, &mut self.renderer);

        self.cache = Some(ui.into_cache());
    }
}

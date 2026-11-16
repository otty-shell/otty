//! The app as the iced runtime runs it, with no window on screen: input
//! goes through the window to `update`, the tasks `update` returns run and
//! their widget operations reach the widgets, and the next frame follows.
//! The widget state carries over from one step to the next, so a command
//! a widget operation queued in a terminal widget runs there on the next
//! frame, as in the running app.

use iced::advanced::clipboard::{Clipboard, Kind};
use iced::advanced::widget::{Operation, operation};
use iced::mouse::{self, Cursor};
use iced::{Event, Point, Rectangle, Size, Task};
use iced_test::runtime::Action;
use iced_test::runtime::user_interface::{Cache, UserInterface};
use iced_test::selector::{Candidate, Selector};

use crate::app::App;
use crate::layout;
use crate::message::{AppEvent, Message};
use crate::testing::frames::headless_renderer;
use crate::testing::task_outputs::ready_actions;

/// The size of the window the app runs in.
const WINDOW: Size = Size::new(1024.0, 768.0);
/// The most frames that follow one input. A widget that asks for something
/// on every frame can not hold a test forever.
const MAX_FRAMES: usize = 4;

/// The clipboard of the window: it keeps the text written to it last.
#[derive(Default)]
struct TestClipboard {
    text: Option<String>,
}

impl Clipboard for TestClipboard {
    fn read(&self, _kind: Kind) -> Option<String> {
        self.text.clone()
    }

    fn write(&mut self, _kind: Kind, contents: String) {
        self.text = Some(contents);
    }
}

/// The app in a window of [`WINDOW`], run step by step by a test.
pub(crate) struct RunningApp {
    app: App,
    renderer: iced::Renderer,
    cache: Option<Cache>,
    cursor: Cursor,
    clipboard: TestClipboard,
}

impl RunningApp {
    /// Return the app.
    pub(crate) fn app(&self) -> &App {
        &self.app
    }

    /// Return the text written to the clipboard last, if any.
    pub(crate) fn clipboard_text(&self) -> Option<&str> {
        self.clipboard.text.as_deref()
    }

    /// Run `app` in a window of [`WINDOW`]. No frame is built yet.
    pub(crate) fn new(mut app: App) -> Self {
        app.screen_size = layout::screen_size_from_window(WINDOW);

        Self {
            app,
            renderer: headless_renderer(),
            cache: Some(Cache::default()),
            cursor: Cursor::Unavailable,
            clipboard: TestClipboard::default(),
        }
    }

    /// Deliver `event` that does not come from the window, such as the
    /// news that a terminal has new content, then the frames after it.
    pub(crate) fn deliver(&mut self, event: AppEvent) {
        self.handle(vec![Message::Internal(event)]);
        self.run_frames();
    }

    /// Move the pointer to `position`, then the frames after it.
    pub(crate) fn move_to(&mut self, position: Point) {
        self.cursor = Cursor::Available(position);
        self.input(&[Event::Mouse(mouse::Event::CursorMoved { position })]);
    }

    /// Press and release `button` where the pointer is, then the frames
    /// after it.
    pub(crate) fn click(&mut self, button: mouse::Button) {
        self.press(button);
        self.release(button);
    }

    /// Press `button` where the pointer is and hold it, then the frames
    /// after it.
    pub(crate) fn press(&mut self, button: mouse::Button) {
        self.input(&[Event::Mouse(mouse::Event::ButtonPressed(button))]);
    }

    /// Release `button` where the pointer is, then the frames after it.
    pub(crate) fn release(&mut self, button: mouse::Button) {
        self.input(&[Event::Mouse(mouse::Event::ButtonReleased(button))]);
    }

    /// Move the pointer to the middle of the text `label` and press the
    /// left button there. Panics when the window does not show `label`.
    pub(crate) fn click_text(&mut self, label: &str) {
        let bounds = self
            .find(label)
            .unwrap_or_else(|| panic!("the window should show {label:?}"));

        self.move_to(bounds.center());
        self.click(mouse::Button::Left);
    }

    /// Type `text` on the keyboard, key by key, then the frames after it.
    pub(crate) fn type_text(&mut self, text: &str) {
        let keys: Vec<Event> = iced_test::simulator::typewrite(text).collect();

        self.input(&keys);
    }

    /// Return where the window shows the text `label`, if it does.
    pub(crate) fn find(&mut self, label: &str) -> Option<Rectangle> {
        let mut find = Selector::find(label);
        self.operate_once(&mut operation::black_box(&mut find));

        match find.finish() {
            operation::Outcome::Some(Some(text)) => Some(text.bounds()),
            _ => None,
        }
    }

    /// Return where the window shows the widget that takes the keyboard
    /// at `point`, such as the terminal of a pane, if one is there.
    pub(crate) fn find_focusable_at(
        &mut self,
        point: Point,
    ) -> Option<Rectangle> {
        let mut find =
            Selector::find(move |candidate: Candidate<'_>| match candidate {
                Candidate::Focusable { bounds, .. }
                    if bounds.contains(point) =>
                {
                    Some(bounds)
                },
                _ => None,
            });
        self.operate_once(&mut operation::black_box(&mut find));

        match find.finish() {
            operation::Outcome::Some(Some(bounds)) => Some(bounds),
            _ => None,
        }
    }

    /// Give `events` to the window, handle what it sends, then the frames
    /// after it.
    fn input(&mut self, events: &[Event]) {
        let messages = self.update_window(events);

        self.handle(messages);
        self.run_frames();
    }

    /// Draw frames, as the window does after an update, until a frame sends
    /// nothing. A widget handles what an operation queued for it on its
    /// next frame.
    fn run_frames(&mut self) {
        for _ in 0..MAX_FRAMES {
            let redraw = Event::Window(iced::window::Event::RedrawRequested(
                iced::time::Instant::now(),
            ));
            let messages = self.update_window(&[redraw]);
            if messages.is_empty() {
                return;
            }

            self.handle(messages);
        }
    }

    /// Build the window of the app as it is now, give it `events` and
    /// return the messages its widgets sent.
    fn update_window(&mut self, events: &[Event]) -> Vec<Message> {
        let mut ui = UserInterface::build(
            self.app.view(),
            WINDOW,
            self.cache.take().expect("the window keeps its cache"),
            &mut self.renderer,
        );
        let mut messages = Vec::new();

        let _ = ui.update(
            events,
            self.cursor,
            &mut self.renderer,
            &mut self.clipboard,
            &mut messages,
        );
        self.cache = Some(ui.into_cache());

        messages
    }

    /// Let the app update with each of `messages`, and run the tasks it
    /// returns.
    fn handle(&mut self, messages: Vec<Message>) {
        for message in messages {
            let task = self.app.update(message);

            self.run(task);
        }
    }

    /// Run what `task` has ready: its messages go to the app, its widget
    /// operations to the window. Work that waits, such as reading a file,
    /// is left out.
    fn run(&mut self, task: Task<Message>) {
        for action in ready_actions(task) {
            match action {
                Action::Output(message) => self.handle(vec![message]),
                Action::Widget(mut pending) => loop {
                    self.operate_once(pending.as_mut());

                    match pending.finish() {
                        operation::Outcome::Chain(next) => pending = next,
                        _ => break,
                    }
                },
                _ => {},
            }
        }
    }

    /// Run `operation` over the window of the app as it is now, once.
    fn operate_once(&mut self, operation: &mut dyn Operation) {
        let mut ui = UserInterface::build(
            self.app.view(),
            WINDOW,
            self.cache.take().expect("the window keeps its cache"),
            &mut self.renderer,
        );

        ui.operate(&self.renderer, operation);
        self.cache = Some(ui.into_cache());
    }
}

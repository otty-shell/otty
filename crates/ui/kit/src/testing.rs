//! Test-only helpers: a folder for the tests that keep reference pictures
//! on disk, the check that a view draws a saved picture, the check that an
//! icon lights up over its whole control, an app that runs for a test, an
//! input that says its name when typed in, a press of Tab, the wheel event
//! the system sends while Shift is held, and the kit's fonts for a drawing
//! test that runs alone.
//!
//! The app crates have the same two last helpers in `otty-app-testing`; the
//! kit can not use that crate, because it depends on the kit.

use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use iced::advanced::clipboard;
use iced::advanced::renderer::Headless;
use iced::advanced::widget::{Operation, operation};
use iced::keyboard::key::Named;
use iced::keyboard::{self, Key, Modifiers, key};
use iced::mouse::{self, ScrollDelta};
use iced::widget::{container, text_input};
use iced::{Element, Event, Point, Rectangle, Renderer, Size, Vector};
use iced_test::runtime::UserInterface;
use iced_test::runtime::user_interface::Cache;
use iced_test::{Selector, simulator};

use crate::fonts;

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

/// Set in the copy of the test program that runs one test alone, so the test
/// knows to go on instead of starting another copy.
const IN_THE_COPY: &str = "OTTY_TEST_ALONE";

/// Where the tested control is placed in the test window.
const ORIGIN: f32 = 40.0;
/// A window for one small control and its tooltip, as [`draws`] places
/// them.
pub(crate) const SMALL_WINDOW: Size = Size::new(240.0, 160.0);
/// The window of a [`LiveApp`].
const APP_WINDOW: Size = Size::new(400.0, 400.0);

/// A fresh folder under the system temp folder for one test, removed when the
/// test ends, also when it fails.
///
/// Each one has its own path, so the end of one test can not take away what
/// another test running at the same time saved.
pub(crate) struct TempDir {
    path: PathBuf,
}

impl TempDir {
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    /// Creates a folder with `name` in its path, to tell where it comes from.
    pub(crate) fn new(name: &str) -> Self {
        let number = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let process = std::process::id();
        let path = std::env::temp_dir()
            .join(format!("otty-test-{name}-{process}-{number}"));

        std::fs::create_dir_all(&path).expect("create test dir");

        Self { path }
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

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
        let mut messages = Vec::new();

        let mut ui = UserInterface::build(
            (self.view)(&self.state),
            self.window,
            self.cache.take().unwrap_or_default(),
            &mut self.renderer,
        );
        let _ = ui.update(
            &events,
            self.cursor,
            &mut self.renderer,
            &mut clipboard::Null,
            &mut messages,
        );
        self.cache = Some(ui.into_cache());

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
        let mut ui = UserInterface::build(
            (self.view)(&self.state),
            self.window,
            self.cache.take().unwrap_or_default(),
            &mut self.renderer,
        );
        let mut find = selector.find();
        ui.operate(&self.renderer, &mut operation::black_box(&mut find));
        self.cache = Some(ui.into_cache());

        match find.finish() {
            operation::Outcome::Some(output) => output,
            _ => None,
        }
    }
}

/// Draws `view` with no pointer, with the pointer on the middle of
/// `control` (a rectangle inside `view`) and with the pointer one pixel
/// inside its top-left corner, away from the icon. The corner picture
/// must equal the middle one (the icon lights up over the whole control)
/// and both must differ from the idle one.
pub(crate) fn assert_icon_lights_up(
    name: &str,
    view: impl Fn() -> Element<'static, ()>,
    control: Rectangle,
) {
    let folder = TempDir::new("icon-hover");
    let reference = folder.path().join(name);
    let middle = control.center();
    let corner = Point::new(control.x + 1.0, control.y + 1.0);

    // Saves the picture of the pointer on the icon.
    assert!(
        draws(&view, SMALL_WINDOW, Some(middle), &reference),
        "{name}: save"
    );

    assert!(
        draws(&view, SMALL_WINDOW, Some(corner), &reference),
        "{name}: the icon must light up with the pointer on the corner"
    );
    assert!(
        !draws(&view, SMALL_WINDOW, None, &reference),
        "{name}: the idle picture must differ from the hovered one"
    );
}

/// One notch of the wheel down while Shift is held, as the system sends it.
///
/// macOS turns it sideways itself, so it arrives as a sideways notch; iced
/// turns it back before applying Shift. Other systems send it as it is.
pub(crate) fn shift_wheel_notch() -> ScrollDelta {
    if cfg!(target_os = "macos") {
        ScrollDelta::Lines { x: -1.0, y: 0.0 }
    } else {
        ScrollDelta::Lines { x: 0.0, y: -1.0 }
    }
}

/// A press and a release of Tab with `modifiers` held, as the system sends
/// them: the press carries the tab character as its text.
pub(crate) fn tab_tap(modifiers: Modifiers) -> [Event; 2] {
    let tab = Key::Named(key::Named::Tab);
    let physical = key::Physical::Code(key::Code::Tab);

    [
        Event::Keyboard(keyboard::Event::KeyPressed {
            key: tab.clone(),
            modified_key: tab.clone(),
            physical_key: physical,
            location: keyboard::Location::Standard,
            modifiers,
            text: Some("\t".into()),
            repeat: false,
        }),
        Event::Keyboard(keyboard::Event::KeyReleased {
            key: tab.clone(),
            modified_key: tab,
            physical_key: physical,
            location: keyboard::Location::Standard,
            modifiers,
        }),
    ]
}

/// Says whether the test `name` runs in its own copy of the test program and
/// should go on. In the normal run it starts the copy with `name` as the
/// filter, checks that exactly one test passed there and returns `false`.
///
/// The kit's fonts belong to the whole process, so a test that loads them
/// (see [`load_kit_fonts`]) would disturb the drawing tests running beside
/// it. `name` must match the test's own name, so the copy runs it and
/// nothing else.
pub(crate) fn is_alone(name: &str) -> bool {
    if std::env::var_os(IN_THE_COPY).is_some() {
        return true;
    }

    let program = std::env::current_exe().expect("the test program");
    let output = std::process::Command::new(program)
        .arg(name)
        .arg("--nocapture")
        .env(IN_THE_COPY, "1")
        .output()
        .expect("the copy should start");
    let said = String::from_utf8_lossy(&output.stdout);
    let complained = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success() && said.contains("1 passed"),
        "{said}{complained}"
    );

    false
}

/// Loads the kit's fonts and lists the computer's variable CJK fonts, so
/// text is measured and drawn as in the app. The fonts stay loaded for the
/// whole process, so a test that loads them runs alone, see [`is_alone`].
pub(crate) fn load_kit_fonts() {
    fonts::register_variable_cjk_faces();

    let settings = iced::Settings {
        fonts: fonts::all_bytes().into_iter().map(Cow::Borrowed).collect(),
        ..iced::Settings::default()
    };
    let empty: Element<'_, ()> = iced::widget::text("").into();

    let _ = iced_test::Simulator::with_settings(settings, empty);
}

/// Says whether `view`, drawn in a `window` with the pointer at `pointer`
/// (or with no pointer), gives the picture saved at `reference`; the first
/// call saves it. The view stands 40px in from the top left corner of the
/// window, and `pointer` is a point of the view, from its top left corner.
pub(crate) fn draws(
    view: &impl Fn() -> Element<'static, ()>,
    window: Size,
    pointer: Option<Point>,
    reference: &Path,
) -> bool {
    let mut ui = iced_test::Simulator::with_size(
        iced::Settings::default(),
        window,
        container(view()).padding(ORIGIN),
    );
    if let Some(point) = pointer {
        let position = point + Vector::new(ORIGIN, ORIGIN);
        ui.point_at(position);
        let _ =
            ui.simulate([Event::Mouse(mouse::Event::CursorMoved { position })]);
    }

    ui.snapshot(&iced::Theme::Dark)
        .expect("view should draw")
        .matches_hash(reference)
        .expect("reference should be readable")
}

/// A text input with the id `name` that sends `on_type(name)` when typed
/// in, so a test sees which field each letter went to.
pub(crate) fn named_input<Message: Clone + 'static>(
    name: &'static str,
    on_type: fn(&'static str) -> Message,
) -> Element<'static, Message> {
    text_input("", "")
        .id(iced::widget::Id::new(name))
        .on_input(move |_| on_type(name))
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_two_folders_then_they_have_their_own_paths() {
        let first = TempDir::new("same");
        let second = TempDir::new("same");

        assert_ne!(first.path(), second.path());
        assert!(first.path().is_dir());
        assert!(second.path().is_dir());
    }

    #[test]
    fn given_a_folder_with_a_file_when_dropped_then_it_is_gone() {
        let folder = TempDir::new("dropped");
        let path = folder.path().to_path_buf();
        std::fs::write(path.join("file"), "x").expect("write");

        drop(folder);

        assert!(!path.exists());
    }

    #[test]
    fn given_a_test_that_fails_then_its_folder_is_gone() {
        let mut path = None;

        let outcome =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let folder = TempDir::new("failing");
                path = Some(folder.path().to_path_buf());
                panic!("the test fails here");
            }));

        assert!(outcome.is_err());
        assert!(!path.expect("the folder was made").exists());
    }
}

//! Input the tests send that `iced_test` has no helper for: a press of Tab,
//! the wheel event the system sends while Shift is held, and an input that
//! says its name when typed in.

use iced::keyboard::{self, Key, Modifiers, key};
use iced::mouse::ScrollDelta;
use iced::widget::text_input;
use iced::{Element, Event};

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

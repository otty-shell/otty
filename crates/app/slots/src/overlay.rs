use iced::Element;

/// The kind of an overlay, which decides how the frame treats the user's
/// input while it is open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayKind {
    /// A menu closes on Esc, on a click outside it and on a shortcut, which
    /// then runs. Keys under it never reach the terminal behind.
    Menu,
    /// A dialog waits for its answer: it ignores shortcuts and clicks
    /// outside it, and Esc means Cancel.
    Dialog,
}

/// A menu or a dialog a feature draws over the window.
///
/// The feature gives only the content and the kind. The frame owns the rest:
/// the layer the overlay lies in, what Esc, an outside click and a shortcut
/// do, and where the keyboard focus goes after it closes.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use iced::widget::text;
/// use otty_app_slots::{Overlay, OverlayKind};
///
/// let menu: Overlay<'_, ()> = Overlay::menu(text("New terminal"));
///
/// assert_eq!(menu.kind(), OverlayKind::Menu);
/// let _content: Element<'_, ()> = menu.into_content();
/// ```
pub struct Overlay<'a, Message> {
    kind: OverlayKind,
    content: Element<'a, Message>,
}

impl<'a, Message> Overlay<'a, Message> {
    /// Return whether this is a menu or a dialog.
    pub fn kind(&self) -> OverlayKind {
        self.kind
    }

    /// A menu with `content`.
    pub fn menu(content: impl Into<Element<'a, Message>>) -> Self {
        Self {
            kind: OverlayKind::Menu,
            content: content.into(),
        }
    }

    /// A dialog with `content`.
    pub fn dialog(content: impl Into<Element<'a, Message>>) -> Self {
        Self {
            kind: OverlayKind::Dialog,
            content: content.into(),
        }
    }

    /// Give up the overlay for the element the frame draws.
    pub fn into_content(self) -> Element<'a, Message> {
        self.content
    }
}

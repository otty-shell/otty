use otty_app_theme::theme::ThemeProps;

use crate::{Feature, MainContent};

/// A feature that draws the main area of a mode.
///
/// The frame owns the main area itself: its place between the side panels,
/// and which feature fills it. In Terminal mode that is the feature of the
/// tab in front (the terminal for a terminal tab), in Containers mode the
/// containers feature. The feature gives only what fills it, drawn from its
/// own state: the tab in front reached that state earlier, through an
/// intent.
///
/// What fills the main area is a [`MainContent`]: a screen, named with the
/// feature's own [`MainArea::Screen`] type so the screens of two features
/// never share their scrolling, or content that scrolls on its own, such as
/// the panes of a terminal.
///
/// # Example
///
/// ```
/// use iced::widget::text;
/// use otty_app_slots::{Feature, MainArea, MainContent};
/// use otty_app_theme::theme::{AppTheme, ThemeProps};
///
/// /// A page that shows the note it was given.
/// struct Note {
///     text: String,
/// }
///
/// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// enum NoteScreen {
///     Page,
/// }
///
/// impl Feature for Note {
///     type Intent = String;
///     type Outcome = ();
///     type Ctx<'a> = ();
///
///     fn reduce(&mut self, text: String, _ctx: ()) -> Vec<()> {
///         self.text = text;
///
///         Vec::new()
///     }
/// }
///
/// impl MainArea for Note {
///     type Screen = NoteScreen;
///
///     fn main_area<'a>(
///         &'a self,
///         _theme: ThemeProps<'a>,
///     ) -> MainContent<'a, String, NoteScreen> {
///         MainContent::Screen {
///             id: NoteScreen::Page,
///             content: text(&self.text).into(),
///         }
///     }
/// }
///
/// let theme = AppTheme::default();
/// let mut note = Note {
///     text: String::new(),
/// };
/// let _ = note.reduce(String::from("Hello"), ());
///
/// let content = note.main_area(ThemeProps::new(&theme));
/// assert!(matches!(content, MainContent::Screen { .. }));
/// ```
pub trait MainArea: Feature {
    /// The feature's own type that names its screens; see
    /// [`MainContent::Screen`].
    type Screen: Clone + PartialEq + 'static;

    /// Return what fills the main area, drawn with `theme`.
    fn main_area<'a>(
        &'a self,
        theme: ThemeProps<'a>,
    ) -> MainContent<'a, Self::Intent, Self::Screen>;
}

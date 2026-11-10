use iced::Element;
use otty_app_theme::theme::ThemeProps;

use crate::Feature;

/// What the frame keeps about one section of the left panel: whether it is
/// open and whether the menu of its header button is open.
///
/// The frame owns both, as it owns which panels are shown and how wide they
/// are: it stacks the sections, shares the panel height between them, and
/// keeps one header menu open at a time for all sections. The feature draws
/// its section with them, as it draws with the theme.
///
/// # Example
///
/// ```
/// use otty_app_slots::SectionState;
///
/// let closed = SectionState::new(false, false);
///
/// assert!(!closed.is_open());
/// assert!(!closed.is_menu_open());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SectionState {
    is_open: bool,
    is_menu_open: bool,
}

impl SectionState {
    /// Return whether the section shows its rows.
    pub fn is_open(&self) -> bool {
        self.is_open
    }

    /// Return whether the menu of the section's header button is open.
    pub fn is_menu_open(&self) -> bool {
        self.is_menu_open
    }

    /// A section that is open or closed, with its header menu open or
    /// closed.
    pub fn new(is_open: bool, is_menu_open: bool) -> Self {
        Self {
            is_open,
            is_menu_open,
        }
    }
}

/// A press in a section of the left panel: on the parts the frame owns (the
/// title that opens and closes the section, the chevron that opens the
/// header menu) or on the feature's own rows and buttons.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SectionMessage<Intent> {
    /// The section title was pressed: open or close the section.
    Toggle,
    /// The chevron of the header button was pressed: open its menu.
    OpenMenu,
    /// A press on the feature's own part of the section.
    Feature(Intent),
}

/// A section of the left panel and the height its rows need, so the frame
/// can share the panel height between the stacked sections: each open
/// section takes the height of its rows while everything fits, and scrolls
/// inside its share when it does not.
///
/// The height is counted from the rows, because iced can not measure a
/// widget before the panel shares its height.
pub struct LeftPanelSection<'a, Intent> {
    content: Element<'a, SectionMessage<Intent>>,
    content_height: f32,
}

impl<'a, Intent> LeftPanelSection<'a, Intent> {
    /// Return the height the rows of the section need to show without
    /// scrolling, without the section header.
    pub fn content_height(&self) -> f32 {
        self.content_height
    }

    /// A section drawn as `content`, whose rows need `content_height`.
    pub fn new(
        content: impl Into<Element<'a, SectionMessage<Intent>>>,
        content_height: f32,
    ) -> Self {
        Self {
            content: content.into(),
            content_height,
        }
    }

    /// Give up the section for the element the frame stacks.
    pub fn into_content(self) -> Element<'a, SectionMessage<Intent>> {
        self.content
    }
}

/// A feature that draws a section of the left panel in Terminal mode, such
/// as the Sessions section of the tabs.
///
/// The frame stacks the sections, keeps which of them are open and which
/// header menu is open, and turns [`SectionMessage::Toggle`] and
/// [`SectionMessage::OpenMenu`] into its own events. The feature draws the
/// whole section, header included, from its own state and the
/// [`SectionState`] the frame gives.
///
/// The view draws only from the feature's own state; outside data reached
/// that state earlier, through an intent.
///
/// # Example
///
/// ```
/// use iced::widget::{button, column, text};
/// use otty_app_slots::{
///     Feature, LeftPanel, LeftPanelSection, SectionMessage, SectionState,
/// };
/// use otty_app_theme::theme::{AppTheme, ThemeProps};
///
/// /// A section with one row per bookmark.
/// struct Bookmarks {
///     names: Vec<String>,
/// }
///
/// #[derive(Debug, Clone)]
/// enum BookmarksIntent {
///     Add(String),
/// }
///
/// impl Feature for Bookmarks {
///     type Intent = BookmarksIntent;
///     type Outcome = ();
///     type Ctx<'a> = ();
///
///     fn reduce(&mut self, intent: BookmarksIntent, _ctx: ()) -> Vec<()> {
///         match intent {
///             BookmarksIntent::Add(name) => self.names.push(name),
///         }
///
///         Vec::new()
///     }
/// }
///
/// impl LeftPanel for Bookmarks {
///     fn left_panel<'a>(
///         &'a self,
///         _theme: ThemeProps<'a>,
///         section: SectionState,
///     ) -> LeftPanelSection<'a, BookmarksIntent> {
///         let title =
///             button(text("Bookmarks")).on_press(SectionMessage::Toggle);
///         let mut rows = column![title];
///         if section.is_open() {
///             for name in &self.names {
///                 rows = rows.push(text(name));
///             }
///         }
///
///         LeftPanelSection::new(rows, 30.0 * self.names.len() as f32)
///     }
/// }
///
/// let theme = AppTheme::default();
/// let mut bookmarks = Bookmarks { names: Vec::new() };
/// let _ = bookmarks.reduce(BookmarksIntent::Add(String::from("otty")), ());
///
/// let section = bookmarks
///     .left_panel(ThemeProps::new(&theme), SectionState::new(true, false));
///
/// assert_eq!(section.content_height(), 30.0);
/// ```
pub trait LeftPanel: Feature {
    /// Return the section, drawn with `theme`, open or closed as `section`
    /// says, with the height its rows need.
    fn left_panel<'a>(
        &'a self,
        theme: ThemeProps<'a>,
        section: SectionState,
    ) -> LeftPanelSection<'a, Self::Intent>;
}

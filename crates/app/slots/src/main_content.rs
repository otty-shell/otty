use iced::Element;
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::screen_area::ScreenArea;

/// What fills the main area. A feature that fills it has to say which of the
/// three it is, so a new screen can not miss the rule of [`ScreenArea`]:
/// a page around a table that scrolls on its own is a
/// [`MainContent::TableScreen`], the panes of a terminal are
/// [`MainContent::ScrollsOnItsOwn`], and anything else is a
/// [`MainContent::Screen`].
///
/// `Screen` is the feature's own type that names its screens, such as an
/// enum with a start card and one value per tab. Each value is one screen to
/// the user: a change of value starts at the top left, and the same value
/// built again keeps its scrolling and focus. Screens of two features never
/// share that state, because their types differ.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use iced::widget::text;
/// use otty_app_slots::MainContent;
/// use otty_app_theme::theme::{AppTheme, ThemeProps};
///
/// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// enum TabsScreen {
///     StartCard,
/// }
///
/// let theme = AppTheme::default();
/// let content: MainContent<'_, (), TabsScreen> = MainContent::Screen {
///     id: TabsScreen::StartCard,
///     content: text("No sessions").into(),
/// };
///
/// let _main_area: Element<'_, ()> =
///     content.into_element(ThemeProps::new(&theme));
/// ```
pub enum MainContent<'a, Message, Screen> {
    /// A screen. It keeps a minimum width and scrolls when there is too
    /// little room.
    Screen {
        /// Which screen this is.
        id: Screen,
        /// What the screen shows.
        content: Element<'a, Message>,
    },
    /// A screen with a table that fills the rest of its height and scrolls
    /// down on its own under its sticky header, such as the sessions of a
    /// project or a Containers list. It keeps the minimum width of a screen and scrolls sideways
    /// below it, by the same [`ScreenArea`], which then never scrolls down.
    TableScreen {
        /// Which screen this is.
        id: Screen,
        /// What the screen shows: its table fills the height that is left.
        content: Element<'a, Message>,
    },
    /// The panes of a terminal. They scroll on their own and fill a height
    /// they are given, and keep no minimum width: a terminal fits its
    /// columns to the room.
    ScrollsOnItsOwn(Element<'a, Message>),
}

impl<'a, Message, Screen> MainContent<'a, Message, Screen>
where
    Message: 'a,
    Screen: Clone + PartialEq + 'static,
{
    /// The element that fills the main area: a screen in its
    /// [`ScreenArea`], one with a table in an area that scrolls only
    /// sideways, or content that scrolls on its own as it is.
    pub fn into_element(self, theme: ThemeProps<'a>) -> Element<'a, Message> {
        match self {
            Self::Screen { id, content } => {
                ScreenArea::new(id, content, theme.theme.ui()).into()
            },
            Self::TableScreen { id, content } => {
                ScreenArea::filling_height(id, content, theme.theme.ui()).into()
            },
            Self::ScrollsOnItsOwn(content) => content,
        }
    }
}

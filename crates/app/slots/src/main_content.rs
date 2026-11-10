use iced::Element;
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::screen_area::ScreenArea;

/// What fills the main area. A feature that fills it has to say which of the
/// two it is, so a new screen can not miss the rule of [`ScreenArea`]:
/// anything that is not a terminal or a table is a [`MainContent::Screen`].
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
    /// The panes of a terminal or the table of a Containers list. They
    /// scroll on their own and fill a height they are given: a table's
    /// sticky header and its rows can not work in an outer scroller, where
    /// the height has no end.
    ScrollsOnItsOwn(Element<'a, Message>),
}

impl<'a, Message, Screen> MainContent<'a, Message, Screen>
where
    Message: 'a,
    Screen: Clone + PartialEq + 'static,
{
    /// The element that fills the main area: a screen in its
    /// [`ScreenArea`], or content that scrolls on its own as it is.
    pub fn into_element(self, theme: ThemeProps<'a>) -> Element<'a, Message> {
        match self {
            Self::Screen { id, content } => {
                ScreenArea::new(id, content, theme.theme.ui()).into()
            },
            Self::ScrollsOnItsOwn(content) => content,
        }
    }
}

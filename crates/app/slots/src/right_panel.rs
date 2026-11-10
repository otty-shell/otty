use iced::Element;
use otty_app_theme::theme::ThemeProps;

use crate::Feature;

/// A feature that draws the right panel of a mode.
///
/// The frame owns the panel itself: whether it is open, its width and the
/// edge that resizes it. The feature gives only what fills it, which takes
/// the room it is given. The screen of each mode picks the feature for the
/// panel, so a mode whose feature does not implement this trait can not put
/// it there.
///
/// The view draws only from the feature's own state; outside data reached
/// that state earlier, through an intent.
///
/// # Example
///
/// ```
/// use iced::widget::text;
/// use otty_app_slots::{Feature, RightPanel};
/// use otty_app_theme::theme::{AppTheme, ThemeProps};
///
/// /// A panel that shows the folder it follows.
/// struct Folder {
///     name: String,
/// }
///
/// #[derive(Debug, Clone)]
/// enum FolderIntent {
///     Follow(String),
/// }
///
/// impl Feature for Folder {
///     type Intent = FolderIntent;
///     type Outcome = ();
///     type Ctx<'a> = ();
///
///     fn reduce(&mut self, intent: FolderIntent, _ctx: ()) -> Vec<()> {
///         match intent {
///             FolderIntent::Follow(name) => self.name = name,
///         }
///
///         Vec::new()
///     }
/// }
///
/// impl RightPanel for Folder {
///     fn right_panel<'a>(
///         &'a self,
///         _theme: ThemeProps<'a>,
///     ) -> iced::Element<'a, FolderIntent> {
///         text(&self.name).into()
///     }
/// }
///
/// let theme = AppTheme::default();
/// let mut folder = Folder {
///     name: String::new(),
/// };
/// let _ = folder.reduce(FolderIntent::Follow(String::from("otty")), ());
///
/// let _panel = folder.right_panel(ThemeProps::new(&theme));
/// ```
pub trait RightPanel: Feature {
    /// Return what fills the right panel, drawn with `theme`.
    fn right_panel<'a>(
        &'a self,
        theme: ThemeProps<'a>,
    ) -> Element<'a, Self::Intent>;
}

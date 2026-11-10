use otty_app_theme::theme::ThemeProps;

use crate::{Component, Overlay};

/// A feature that draws a menu or a dialog over the window.
///
/// A feature has at most one overlay open at a time, so it gives one or
/// none. The frame puts it into the feature's layer of the one layer order
/// and treats it by its [`Overlay::kind`]: the rules for Esc, an outside
/// click and a shortcut belong to the frame, not to the feature.
///
/// The view draws only from the feature's own state; outside data reached
/// that state earlier, through an intent.
///
/// What floats over the window without taking the input, such as the error
/// toasts, is no overlay: the feature gives it through a method of its own,
/// and the frame draws it under every overlay.
///
/// # Example
///
/// ```
/// use iced::widget::text;
/// use otty_app_slots::{Component, Overlay, OverlayKind, Overlays};
/// use otty_app_theme::theme::{AppTheme, ThemeProps};
///
/// /// A menu that opens and closes.
/// struct Menu {
///     is_open: bool,
/// }
///
/// #[derive(Debug, Clone)]
/// enum MenuIntent {
///     Toggle,
/// }
///
/// impl Component for Menu {
///     type Intent = MenuIntent;
///     type Outcome = ();
///     type Ctx<'a> = ();
///
///     fn reduce(&mut self, intent: MenuIntent, _ctx: ()) -> Vec<()> {
///         match intent {
///             MenuIntent::Toggle => self.is_open = !self.is_open,
///         }
///
///         Vec::new()
///     }
/// }
///
/// impl Overlays for Menu {
///     fn overlay<'a>(
///         &'a self,
///         _theme: ThemeProps<'a>,
///     ) -> Option<Overlay<'a, MenuIntent>> {
///         self.is_open.then(|| Overlay::menu(text("New terminal")))
///     }
/// }
///
/// let theme = AppTheme::default();
/// let mut menu = Menu { is_open: false };
/// assert!(menu.overlay(ThemeProps::new(&theme)).is_none());
///
/// let _ = menu.reduce(MenuIntent::Toggle, ());
/// let open = menu
///     .overlay(ThemeProps::new(&theme))
///     .map(|open| open.kind());
///
/// assert_eq!(open, Some(OverlayKind::Menu));
/// ```
pub trait Overlays: Component {
    /// Return the menu or the dialog the feature has open now, drawn with
    /// `theme`, or `None` while nothing is open.
    fn overlay<'a>(
        &'a self,
        theme: ThemeProps<'a>,
    ) -> Option<Overlay<'a, Self::Intent>>;
}

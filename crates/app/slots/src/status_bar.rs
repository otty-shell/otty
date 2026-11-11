use otty_ui_kit::components::status_bar::StatusItem;

use crate::Component;

/// A feature that fills the left side of the status bar of a mode.
///
/// The frame owns the bar itself: it draws it at the bottom of the window,
/// puts the feature's items on the left, and adds its own items on the right,
/// such as the sessions button and the OTTY version. Those right items
/// belong to other parts of the app, so the frame gathers them, as it does
/// any view that holds buttons of several features. The feature gives only
/// its own items, drawn from its own state.
///
/// # Example
///
/// ```
/// use otty_app_slots::{Component, StatusBar};
/// use otty_ui_kit::components::status_bar::StatusItem;
///
/// /// A feature that counts its jobs.
/// struct Jobs {
///     count: usize,
/// }
///
/// impl Component for Jobs {
///     type Intent = usize;
///     type Outcome = ();
///     type Ctx<'a> = ();
///
///     fn reduce(&mut self, count: usize, _ctx: ()) -> Vec<()> {
///         self.count = count;
///
///         Vec::new()
///     }
/// }
///
/// impl StatusBar for Jobs {
///     fn status_bar(&self) -> Vec<StatusItem<usize>> {
///         vec![StatusItem::text(format!("{} jobs", self.count))]
///     }
/// }
///
/// let mut jobs = Jobs { count: 0 };
/// let _ = jobs.reduce(3, ());
///
/// assert!(matches!(
///     jobs.status_bar().as_slice(),
///     [StatusItem::Text(text)] if text == "3 jobs"
/// ));
/// ```
pub trait StatusBar: Component {
    /// Return the items of the left side of the status bar, in order.
    fn status_bar(&self) -> Vec<StatusItem<Self::Intent>>;
}

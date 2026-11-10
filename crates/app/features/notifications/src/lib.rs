//! The notifications feature of the OTTY app: it tells the user about errors
//! with toasts at the top of the window and keeps the last ones for the list
//! under the bell in the header.
//!
//! Other features report an error through their own outcome, and the binary
//! turns it into [`NotificationsIntent::Push`]. The feature does no I/O: the
//! binary reads the clock into [`NotificationsCtx`] and runs the toast timer
//! that [`Notifications::toast_timer`] asks for.
//!
//! # Example
//!
//! ```
//! use std::time::{Duration, Instant};
//!
//! use otty_app_notifications::{
//!     Notifications, NotificationsCtx, NotificationsIntent,
//!     NotificationsOutcome,
//! };
//! use otty_app_slots::Feature;
//!
//! let mut notifications = Notifications::new();
//! let now = Instant::now();
//!
//! let pushed = notifications.reduce(
//!     NotificationsIntent::Push(String::from("Could not save the modes")),
//!     NotificationsCtx { now },
//! );
//! assert!(pushed.is_empty());
//! assert_eq!(notifications.unseen_count(), 1);
//!
//! let opened = notifications.reduce(
//!     NotificationsIntent::OpenList,
//!     NotificationsCtx {
//!         now: now + Duration::from_secs(1),
//!     },
//! );
//! assert_eq!(opened, vec![NotificationsOutcome::ListOpened]);
//! assert_eq!(notifications.unseen_count(), 0);
//! ```

mod ctx;
mod event;
mod model;
mod reducer;
mod state;
mod view;

use std::time::Duration;

pub use ctx::NotificationsCtx;
pub use event::{NotificationsIntent, NotificationsOutcome};
use iced::Element;
use model::{NotificationListViewModel, ToastStackViewModel};
use otty_app_slots::{Feature, Overlay, Overlays};
use otty_app_theme::theme::ThemeProps;
use state::NotificationsState;
use view::{notification_list, toast_stack};

/// Time between two ticks of the toast timer: four times a second, so a
/// toast goes away about five seconds after it came.
const TOAST_TICK: Duration = Duration::from_millis(250);

/// The notifications feature: the error toasts and the list under the bell.
///
/// The list is kept in memory only and holds the last 100 notifications.
///
/// # Example
///
/// ```
/// use otty_app_notifications::Notifications;
///
/// let notifications = Notifications::new();
///
/// assert!(!notifications.is_list_open());
/// assert_eq!(notifications.unseen_count(), 0);
/// assert_eq!(notifications.toast_timer(), None);
/// ```
pub struct Notifications {
    state: NotificationsState,
}

impl Notifications {
    /// Return how many notifications came since the list was last opened,
    /// for the badge on the bell.
    pub fn unseen_count(&self) -> usize {
        self.state.unseen()
    }

    /// Return whether the notifications list is open.
    pub fn is_list_open(&self) -> bool {
        self.state.is_list_open()
    }

    /// Return how often the toast timer has to tick, or `None` while no
    /// toast shows and nothing has to run. Each tick is a
    /// [`NotificationsIntent::Tick`].
    ///
    /// The binary runs the timer: the iced timer needs an async runtime,
    /// which a feature crate does not take.
    pub fn toast_timer(&self) -> Option<Duration> {
        self.state.has_toasts().then_some(TOAST_TICK)
    }

    /// Create the feature with no notification.
    pub fn new() -> Self {
        Self {
            state: NotificationsState::default(),
        }
    }

    /// Render the toasts at the top center of the area the frame gives,
    /// newest on top, or `None` while no toast shows.
    ///
    /// The toasts are no overlay: Esc leaves them, they take no keyboard
    /// focus, and a dialog lies over them. Only the toasts take presses, so
    /// presses beside them reach the layers below.
    pub fn toasts<'a>(
        &'a self,
        theme: ThemeProps<'a>,
    ) -> Option<Element<'a, NotificationsIntent>> {
        toast_stack::view(toast_stack::ToastStackProps {
            vm: ToastStackViewModel::from_state(&self.state),
            theme,
        })
    }
}

impl Default for Notifications {
    fn default() -> Self {
        Self::new()
    }
}

impl Feature for Notifications {
    type Intent = NotificationsIntent;
    type Outcome = NotificationsOutcome;
    type Ctx<'a> = NotificationsCtx;

    fn reduce(
        &mut self,
        intent: NotificationsIntent,
        ctx: NotificationsCtx,
    ) -> Vec<NotificationsOutcome> {
        reducer::reduce(&mut self.state, intent, ctx)
    }
}

impl Overlays for Notifications {
    /// The open list, as a menu at the top right of the area the frame
    /// gives, under the bell. A press outside it closes it.
    fn overlay<'a>(
        &'a self,
        theme: ThemeProps<'a>,
    ) -> Option<Overlay<'a, NotificationsIntent>> {
        let vm = NotificationListViewModel::from_state(&self.state)?;

        Some(Overlay::menu(notification_list::view(
            notification_list::NotificationListProps { vm, theme },
        )))
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use otty_app_slots::OverlayKind;
    use otty_app_theme::theme::ThemeManager;

    use super::*;

    fn at(now: Instant) -> NotificationsCtx {
        NotificationsCtx { now }
    }

    #[test]
    fn without_a_toast_the_timer_does_not_run() {
        let notifications = Notifications::new();

        assert_eq!(notifications.toast_timer(), None);
    }

    #[test]
    fn while_a_toast_shows_the_timer_runs_and_stops_after_it() {
        let mut notifications = Notifications::new();
        let pushed = Instant::now();
        let _ = notifications.reduce(
            NotificationsIntent::Push(String::from("disk full")),
            at(pushed),
        );
        let while_shown = notifications.toast_timer();

        let _ = notifications.reduce(
            NotificationsIntent::Tick,
            at(pushed + Duration::from_secs(5)),
        );

        assert_eq!(while_shown, Some(TOAST_TICK));
        assert_eq!(notifications.toast_timer(), None);
    }

    #[test]
    fn a_toast_is_drawn_only_while_it_shows() {
        let themes = ThemeManager::new();
        let theme = ThemeProps::new(themes.current());
        let mut notifications = Notifications::new();
        let without = notifications.toasts(theme).is_none();

        let _ = notifications.reduce(
            NotificationsIntent::Push(String::from("disk full")),
            at(Instant::now()),
        );

        assert!(without);
        assert!(notifications.toasts(theme).is_some());
    }

    /// The open list is a menu: the frame closes it on Esc, an outside
    /// click and a shortcut.
    #[test]
    fn the_open_list_is_a_menu_overlay() {
        let themes = ThemeManager::new();
        let theme = ThemeProps::new(themes.current());
        let mut notifications = Notifications::new();
        let closed = notifications.overlay(theme).is_none();

        let _ = notifications
            .reduce(NotificationsIntent::OpenList, at(Instant::now()));

        let kind = notifications.overlay(theme).map(|open| open.kind());
        assert!(closed);
        assert_eq!(kind, Some(OverlayKind::Menu));
    }
}

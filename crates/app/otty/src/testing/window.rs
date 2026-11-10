//! A new app for tests, built without reading files, and the states the
//! tests start from.

use otty_app_notifications::NotificationsIntent;
use otty_app_theme::theme::BuiltinTheme;

use crate::app::App;
use crate::dispatch;
use crate::message::AppEvent;
use crate::modes::types::DEFAULT_PINNED;

/// A shell without OTTY shell integration, so building its session writes
/// no files.
const TEST_SHELL_PATH: &str = "/opt/otty-test/otty-test-shell";

/// A new app as on a first start: the default theme, Terminal mode and the
/// default pinned modes. It reads no settings or modes file.
pub(crate) fn app() -> App {
    App::with_saved(BuiltinTheme::default(), TEST_SHELL_PATH, &DEFAULT_PINNED)
}

/// Tell the user about an error, as the binary does for a feature's
/// outcome: a toast shows `text`.
pub(crate) fn show_toast(app: &mut App, text: &str) {
    let _none = dispatch::dispatch(
        app,
        AppEvent::Notifications(NotificationsIntent::Push(String::from(text))),
    );
}

/// Open the notifications list, as the bell does.
pub(crate) fn open_notification_list(app: &mut App) {
    let _focus = dispatch::dispatch(
        app,
        AppEvent::Notifications(NotificationsIntent::OpenList),
    );
}

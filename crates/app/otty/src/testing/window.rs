//! A new app for tests, built without reading files, and the states the
//! tests start from.

use otty_app_i18n::Locale;
use otty_app_notifications::NotificationsIntent;
use otty_app_palette::PaletteIntent;
use otty_app_settings::{LanguageSetting, SettingsData, SettingsIntent};

use crate::app::App;
use crate::dispatch;
use crate::message::AppEvent;
use crate::modes::types::DEFAULT_PINNED;

/// A shell without OTTY shell integration, so building its session writes
/// no files.
pub(crate) const TEST_SHELL_PATH: &str = "/opt/otty-test/otty-test-shell";

/// Saved settings for tests: the defaults, with a shell whose session
/// writes no files, and English, so applying them never switches the
/// language the other tests read their texts in.
pub(crate) fn test_settings() -> SettingsData {
    let mut saved = SettingsData::default();
    saved.set_terminal_shell(String::from(TEST_SHELL_PATH));
    saved.set_language(LanguageSetting::Fixed(Locale::En));

    saved
}

/// A new app as on a first start: the default theme, Terminal mode and the
/// default pinned modes. It reads no settings or modes file.
pub(crate) fn app() -> App {
    App::with_saved(test_settings(), &DEFAULT_PINNED)
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

/// Open the command palette, as its shortcut does.
pub(crate) fn open_palette(app: &mut App) {
    let _focus =
        dispatch::dispatch(app, AppEvent::Palette(PaletteIntent::Open));
}

/// Open the Settings dialog, as the gear does. The read of the settings
/// file it asks for is not run.
pub(crate) fn open_settings(app: &mut App) {
    let _read =
        dispatch::dispatch(app, AppEvent::Settings(SettingsIntent::Open));
}

/// Type `shell` in the Shell field of the open Settings dialog: an edit
/// that is not saved.
pub(crate) fn edit_the_shell(app: &mut App, shell: &str) {
    let _none = dispatch::dispatch(
        app,
        AppEvent::Settings(SettingsIntent::ShellChanged(String::from(shell))),
    );
}

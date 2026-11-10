//! A new app for tests, built without reading files, and the states the
//! tests start from.

use otty_app_i18n::Locale;
use otty_app_notifications::NotificationsIntent;
use otty_app_palette::PaletteIntent;
use otty_app_quick_launch::{
    QuickLaunchFile, QuickLaunchIntent, QuickLaunchSetupOutcome, SshSecretKind,
    SshSecretRequest,
};
use otty_app_settings::{LanguageSetting, SettingsData, SettingsIntent};
use otty_app_tabs::TabsIntent;
use otty_app_terminal::TerminalIntent;

use crate::app::App;
use crate::dispatch;
use crate::message::AppEvent;
use crate::modes::types::DEFAULT_PINNED;

/// A real shell without OTTY shell integration: building its session writes
/// no files, and a terminal tab of the tests starts it.
#[cfg(unix)]
pub(crate) const TEST_SHELL_PATH: &str = "/bin/sh";
/// A real shell without OTTY shell integration: building its session writes
/// no files, and a terminal tab of the tests starts it.
#[cfg(target_os = "windows")]
pub(crate) const TEST_SHELL_PATH: &str = "cmd.exe";

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
    app_with_launches(QuickLaunchFile::default())
}

/// A new app as [`app`] with `launches` read at start.
pub(crate) fn app_with_launches(launches: QuickLaunchFile) -> App {
    App::with_saved(test_settings(), &DEFAULT_PINNED, launches, None)
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

/// Open a terminal tab, as New terminal does: its shell starts and its only
/// pane is the focused pane of the tab in front.
pub(crate) fn open_terminal_tab(app: &mut App) {
    let _scroll =
        dispatch::dispatch(app, AppEvent::Tabs(TabsIntent::OpenTerminalTab));
}

/// Open a terminal tab, as New terminal does, and return its id.
pub(crate) fn open_shell_tab(app: &mut App) -> u64 {
    open_terminal_tab(app);

    app.tabs.active_tab_id().expect("the shell tab should open")
}

/// Open a shell tab and the context menu of its pane, as a right press on
/// the pane does.
pub(crate) fn open_pane_menu(app: &mut App) {
    let tab_id = open_shell_tab(app);
    let (_, pane) = app.terminal.focused_pane().expect("the pane of the tab");
    let terminal_id = app
        .terminal
        .focused_terminal_id()
        .expect("the terminal of the pane");

    let _keyboard = dispatch::dispatch(
        app,
        AppEvent::Terminal(TerminalIntent::OpenContextMenu {
            tab_id,
            pane,
            terminal_id,
        }),
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

/// Open the SSH secret prompt, as the check of an SSH launch that needs a
/// password does.
pub(crate) fn open_secret_prompt(app: &mut App) {
    let launch = serde_json::from_value(serde_json::json!({
        "title": "laptop",
        "spec": {"type": "Ssh", "ssh": {"host": "192.168.1.20", "port": 22,
            "user": "ubuntu", "identity_file": null, "extra_args": []}}
    }))
    .expect("the launch should parse");
    let asked = QuickLaunchSetupOutcome::SecretRequired {
        path: vec![String::from("laptop")],
        launch_id: 1,
        command: Box::new(launch),
        request: SshSecretRequest::new(
            SshSecretKind::Password,
            String::from("ubuntu@192.168.1.20:22"),
            false,
        ),
    };

    let _focus = dispatch::dispatch(
        app,
        AppEvent::QuickLaunch(QuickLaunchIntent::SetupCompleted(asked)),
    );
}

/// Open the context menu of the Launches tree, as a right press on its
/// empty area does.
pub(crate) fn open_launch_menu(app: &mut App) {
    let _keyboard = dispatch::dispatch(
        app,
        AppEvent::QuickLaunch(QuickLaunchIntent::BackgroundRightClicked),
    );
}

/// Start naming a new folder in the Launches tree, as New folder does.
pub(crate) fn start_inline_edit(app: &mut App) {
    let _focus = dispatch::dispatch(
        app,
        AppEvent::QuickLaunch(QuickLaunchIntent::HeaderCreateFolder),
    );
}

//! Open a terminal tab that runs a real shell, for tests that need a pane.

use iced::{Point, Size};
use otty_ui_term::settings::{LocalSessionOptions, SessionKind, Settings};

use crate::app::App;
use crate::widgets::tabs::TabsCtx;
use crate::widgets::tabs::event::TabsIntent;
use crate::widgets::terminal_workspace::TerminalWorkspaceCtx;
use crate::widgets::terminal_workspace::event::TerminalWorkspaceIntent;
use crate::widgets::terminal_workspace::types::TerminalKind;

#[cfg(unix)]
const TEST_SHELL_PATH: &str = "/bin/sh";
#[cfg(target_os = "windows")]
const TEST_SHELL_PATH: &str = "cmd.exe";

/// Open a shell tab with one real terminal, make it the active tab and
/// return its id. Its only pane has keyboard focus.
pub(crate) fn open_shell_tab(app: &mut App) -> u64 {
    let _ = app
        .widgets
        .tabs
        .reduce(TabsIntent::OpenTerminalTab, &TabsCtx { shell_name: "sh" });
    let tab_id = app
        .widgets
        .tabs
        .active_tab_id()
        .expect("shell tab should open");

    let mut settings = Settings::default();
    settings.backend =
        settings
            .backend
            .clone()
            .with_session(SessionKind::from_local_options(
                LocalSessionOptions::default().with_program(TEST_SHELL_PATH),
            ));
    let ctx = TerminalWorkspaceCtx {
        active_tab_id: Some(tab_id),
        pane_grid_size: Size::new(800.0, 600.0),
        screen_size: Size::new(800.0, 600.0),
        sidebar_cursor: Point::ORIGIN,
        equalize_panes: true,
    };

    let _ = app.widgets.terminal_workspace.reduce(
        TerminalWorkspaceIntent::OpenTab {
            tab_id,
            default_title: String::from("sh"),
            settings: Box::new(settings),
            kind: TerminalKind::Shell,
            sync_explorer: false,
        },
        &ctx,
    );

    tab_id
}

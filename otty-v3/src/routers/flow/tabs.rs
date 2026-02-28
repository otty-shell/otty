use std::path::{Path, PathBuf};

use iced::Task;
use otty_ui_term::settings::{LocalSessionOptions, SessionKind, Settings};

use crate::app::{App, AppEvent};
use crate::routers;
use crate::widgets::tabs::TabsCommand;
use crate::widgets::terminal_workspace::services::terminal_settings_for_session;

/// Orchestrate opening a new terminal tab.
///
/// Allocates a terminal_id from the terminal workspace widget
/// and uses the configured shell name as the tab title.
pub(crate) fn open_terminal_tab(app: &mut App) -> Task<AppEvent> {
    let terminal_id = app.widgets.terminal_workspace.allocate_terminal_id();
    let title = app.shell_session.name().to_string();

    routers::tabs::route_command(
        app,
        TabsCommand::OpenTerminalTab { terminal_id, title },
    )
}

/// Orchestrate opening a new settings tab.
pub(crate) fn open_settings_tab(app: &mut App) -> Task<AppEvent> {
    routers::tabs::route_command(app, TabsCommand::OpenSettingsTab)
}

/// Orchestrate opening a file in a new terminal tab (from explorer).
pub(crate) fn open_file_terminal_tab(
    app: &mut App,
    file_path: PathBuf,
) -> Task<AppEvent> {
    let Some(settings) = editor_terminal_settings(app, &file_path) else {
        return Task::none();
    };

    let terminal_id = app.widgets.terminal_workspace.allocate_terminal_id();
    let file_display = file_path.display();
    let title = file_path
        .file_name()
        .and_then(|name| name.to_str())
        .map(ToString::to_string)
        .unwrap_or_else(|| format!("{file_display}"));

    routers::tabs::route_command(
        app,
        TabsCommand::OpenCommandTab {
            terminal_id,
            title,
            settings: Box::new(settings),
        },
    )
}

fn editor_terminal_settings(app: &App, file_path: &Path) -> Option<Settings> {
    let editor_command = app.widgets.settings.terminal_editor().trim();
    let (program, mut args) = parse_command_line(editor_command)?;

    args.push(file_path.to_string_lossy().into_owned());

    let mut options = LocalSessionOptions::default()
        .with_program(&program)
        .with_args(args);

    if let Some(parent) = file_path.parent() {
        options = options.with_working_directory(parent.into());
    }

    let session = SessionKind::from_local_options(options);
    Some(terminal_settings_for_session(
        &app.terminal_settings,
        session,
    ))
}

fn parse_command_line(input: &str) -> Option<(String, Vec<String>)> {
    let parts = match shell_words::split(input) {
        Ok(parts) => parts,
        Err(err) => {
            log::warn!("default editor parse failed: {err}");
            return None;
        },
    };
    let Some((program, args)) = parts.split_first() else {
        log::warn!("default editor command is empty");
        return None;
    };

    Some((program.clone(), args.to_vec()))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::open_file_terminal_tab;
    use crate::app::App;
    use crate::widgets::settings::SettingsCommand;
    use crate::widgets::tabs::model::TabContent;

    #[test]
    fn given_valid_editor_command_when_opening_file_then_command_tab_is_added()
    {
        let (mut app, _) = App::new();
        let _ = crate::routers::settings::route_command(
            &mut app,
            SettingsCommand::EditorChanged(String::from("nvim -u NORC")),
        );

        let _ = open_file_terminal_tab(&mut app, PathBuf::from("/tmp/main.rs"));

        assert_eq!(app.widgets.tabs.len(), 1);
        assert_eq!(app.widgets.tabs.active_tab_title(), Some("main.rs"));
        assert_eq!(
            app.widgets.tabs.active_tab_content(),
            Some(TabContent::Terminal),
        );
    }

    #[test]
    fn given_invalid_editor_command_when_opening_file_then_no_tab_is_added() {
        let (mut app, _) = App::new();
        let _ = crate::routers::settings::route_command(
            &mut app,
            SettingsCommand::EditorChanged(String::from("nvim '")),
        );

        let _ = open_file_terminal_tab(&mut app, PathBuf::from("/tmp/main.rs"));

        assert_eq!(app.widgets.tabs.len(), 0);
    }
}

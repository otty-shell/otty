use std::path::{Path, PathBuf};

use iced::window::Direction;
use iced::{Task, window};
use otty_ui_term::settings::{LocalSessionOptions, SessionKind, Settings};

use crate::app::App;
use crate::layout::screen_size_from_window;
use crate::widgets::chrome::ChromeEvent;
use crate::widgets::explorer::ExplorerEvent;
use crate::widgets::quick_launch::QuickLaunchEvent;
use crate::widgets::settings::SettingsEvent;
use crate::widgets::sidebar::SidebarEvent;
use crate::widgets::tabs::{TabsEvent, TabsUiEvent};
use crate::widgets::terminal_workspace::services::terminal_settings_for_session;
use crate::widgets::terminal_workspace::{
    TerminalWorkspaceEvent, TerminalWorkspaceUiEvent,
};

pub(crate) mod chrome;
pub(crate) mod explorer;
pub(crate) mod quick_launch;
pub(crate) mod settings;
pub(crate) mod sidebar;
pub(crate) mod tabs;
pub(crate) mod terminal_workspace;

/// App-wide events that drive the root update loop.
#[derive(Clone)]
pub(crate) enum AppEvent {
    IcedReady,
    // Sidebar widget
    Sidebar(SidebarEvent),
    // Chrome widget
    Chrome(ChromeEvent),
    // Tabs widget
    Tabs(TabsEvent),
    // Quick Launch widget
    QuickLaunch(QuickLaunchEvent),
    // Terminal Workspace widget
    TerminalWorkspace(TerminalWorkspaceEvent),
    // Explorer widget
    Explorer(ExplorerEvent),
    // Settings widget
    Settings(SettingsEvent),
    // Cross-widget workflows
    OpenTerminalTab,
    OpenFileTerminalTab { file_path: std::path::PathBuf },
    CloseTab { tab_id: u64 },
    // Direct operations
    SyncTerminalGridSizes,
    Keyboard(iced::keyboard::Event),
    Window(iced::window::Event),
    ResizeWindow(Direction),
}

pub(crate) fn handle(app: &mut App, event: AppEvent) -> Task<AppEvent> {
    match event {
        AppEvent::IcedReady => open_terminal_tab(app),
        AppEvent::Sidebar(event) => sidebar::handle(app, event),
        AppEvent::Chrome(event) => chrome::handle(app, event),
        AppEvent::Tabs(event) => tabs::handle(app, event),
        AppEvent::QuickLaunch(event) => quick_launch::handle(app, event),
        AppEvent::TerminalWorkspace(event) => {
            terminal_workspace::handle(app, event)
        },
        AppEvent::Explorer(event) => explorer::handle(app, event),
        AppEvent::Settings(event) => settings::handle(app, event),
        AppEvent::OpenTerminalTab => open_terminal_tab(app),
        AppEvent::OpenFileTerminalTab { file_path } => {
            open_file_terminal_tab(app, file_path)
        },
        AppEvent::CloseTab { tab_id } => {
            Task::done(AppEvent::Tabs(TabsEvent::Ui(TabsUiEvent::CloseTab {
                tab_id,
            })))
        },
        AppEvent::SyncTerminalGridSizes => {
            Task::done(AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Ui(
                TerminalWorkspaceUiEvent::SyncPaneGridSize,
            )))
        },
        AppEvent::Keyboard(_event) => Task::none(),
        AppEvent::Window(iced::window::Event::Resized(size)) => {
            handle_resize(app, size)
        },
        AppEvent::ResizeWindow(dir) => {
            window::latest().and_then(move |id| window::drag_resize(id, dir))
        },
        AppEvent::Window(_) => Task::none(),
    }
}

fn handle_resize(app: &mut App, size: iced::Size) -> Task<AppEvent> {
    app.window_size = size;
    app.state.window_size = size;
    app.state.set_screen_size(screen_size_from_window(size));

    Task::done(AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Ui(
        TerminalWorkspaceUiEvent::SyncPaneGridSize,
    )))
}

fn open_terminal_tab(app: &mut App) -> Task<AppEvent> {
    let terminal_id = app.widgets.terminal_workspace.allocate_terminal_id();
    let title = app.shell_session.name().to_string();

    Task::done(AppEvent::Tabs(TabsEvent::Ui(
        TabsUiEvent::OpenTerminalTab { terminal_id, title },
    )))
}

fn open_file_terminal_tab(
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

    Task::done(AppEvent::Tabs(TabsEvent::Ui(TabsUiEvent::OpenCommandTab {
        terminal_id,
        title,
        settings: Box::new(settings),
    })))
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
    use super::parse_command_line;

    #[test]
    fn given_valid_command_line_when_parsed_then_program_and_args_are_returned()
    {
        let parsed =
            parse_command_line("nvim -u NORC").expect("command should parse");
        assert_eq!(parsed.0, "nvim");
        assert_eq!(parsed.1, vec![String::from("-u"), String::from("NORC")]);
    }

    #[test]
    fn given_invalid_command_line_when_parsed_then_none_is_returned() {
        assert!(parse_command_line("nvim '").is_none());
    }
}

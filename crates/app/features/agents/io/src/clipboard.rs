//! The clipboard of Agents mode: the command that continues a session and
//! the id of a session.

use iced::Task;
use otty_agents::AgentCommand;
use otty_app_agents::AgentsIntent;

/// Put the line that runs `command` in a shell on the clipboard: its
/// variables, then the program and its arguments, each quoted the way the
/// agent tab quotes them, such as
/// `CODEX_HOME='/home/user/.codex-work' 'codex' 'resume' 'a1'`.
pub fn copy_command(command: &AgentCommand) -> Task<AgentsIntent> {
    let line = otty_app_shell::command_line(
        command.program(),
        command.args(),
        command.env(),
    );

    iced::clipboard::write(line)
}

/// Put `text` on the clipboard.
pub fn copy_text(text: String) -> Task<AgentsIntent> {
    iced::clipboard::write(text)
}

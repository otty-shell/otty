mod errors;
mod event;
mod model;
mod services;
mod state;

#[allow(unused_imports)]
pub(crate) use errors::TerminalError;
pub(crate) use event::{
    TerminalEvent, shell_cwd_for_active_tab, shell_cwd_for_terminal_event,
    terminal_reducer,
};
pub(crate) use model::{ShellSession, TerminalEntry, TerminalKind};
pub(crate) use services::{
    fallback_shell_session_with_shell, setup_shell_session_with_shell,
    terminal_settings_for_session,
};
pub(crate) use state::TerminalState;

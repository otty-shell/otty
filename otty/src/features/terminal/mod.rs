mod errors;
mod event;
mod model;
mod services;
mod state;

pub(crate) use event::{TerminalEvent, terminal_reducer};
pub(crate) use model::{ShellSession, TerminalEntry, TerminalKind};
pub(crate) use services::{
    fallback_shell_session_with_shell, setup_shell_session_with_shell,
    shell_cwd_for_active_tab, terminal_settings_for_session,
};
pub(crate) use state::{TerminalState, TerminalTabState};

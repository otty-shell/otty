mod errors;
mod event;
mod feature;
mod model;
mod services;
mod state;

pub(crate) use event::TerminalEvent;
pub(crate) use feature::{
    TerminalCtx, TerminalFeature, shell_cwd_for_active_tab,
};
pub(crate) use model::{ShellSession, TerminalEntry, TerminalKind};
pub(crate) use services::{
    fallback_shell_session_with_shell, setup_shell_session_with_shell,
    terminal_settings_for_session,
};
pub(crate) use state::TerminalState;

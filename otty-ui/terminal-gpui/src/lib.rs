#![doc = include_str!("../README.md")]

mod actions;
mod appearance;
mod backend;
mod behavior;
mod bindings;
mod config;
mod error;
mod event;
mod font;
mod geometry;
mod input;
mod render_runs;
mod terminal;
mod terminal_element;
mod theme;

pub use actions::{
    ClearSelection, Copy, Paste, ScrollPageDown, ScrollPageUp, ScrollToBottom,
    ScrollToTop, SelectAll,
};
pub use appearance::TerminalAppearance;
pub use backend::{
    BackendGeneration, BackendSession, BackendState, LocalBackend,
    LocalOptions, SshBackend, SshOptions, TerminalBackend,
};
pub use behavior::{
    BellPolicy, BlockUiMode, ContextMenuPolicy, LinkPolicy, TerminalBehavior,
};
pub use bindings::{BindingAction, TerminalBinding, TerminalBindings};
pub use config::{TerminalConfig, TerminalConfigBuilder};
pub use error::{BackendError, ConfigError, OperationError};
pub use event::{BlockId, BlockTextPart, CopySource, HitTarget, TerminalEvent};
pub use font::TerminalFont;
pub use geometry::{CellMetrics, TerminalGeometry};
pub use terminal::Terminal;
pub use theme::{ColorPalette, TerminalColor, TerminalTheme};

use std::path::PathBuf;

use otty_agents::AgentId;

/// What the user does in the profile dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileDialogIntent {
    /// Another agent was picked for the new profile.
    AgentPicked(AgentId),
    /// The name changed.
    NameChanged(String),
    /// Another way to give the config folder was picked.
    FolderChoicePicked(FolderChoice),
    /// One of the folders OTTY found was picked.
    FoundFolderPicked(PathBuf),
    /// The path of the config folder changed: the typed path or the path
    /// of the new folder, whichever is picked.
    PathChanged(String),
    /// The path of the agent's program changed.
    ProgramChanged(String),
    /// Add or Save was pressed.
    Submitted,
    /// Cancel was pressed, or Esc.
    Cancelled,
}

/// How the config folder of a new profile is given.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FolderChoice {
    /// One of the folders OTTY found in the home folder.
    Found,
    /// A path the user types, of a folder that already exists.
    Typed,
    /// A new folder OTTY creates, at the proposed path or one the user
    /// types.
    Create,
}

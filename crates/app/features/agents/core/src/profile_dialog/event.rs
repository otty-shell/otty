use otty_agents::AgentId;

use crate::folder_field::FolderFieldIntent;

/// What the user does in the profile dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileDialogIntent {
    /// The agent field was pressed, or a press landed outside its open
    /// list: open the list of agents, or close it.
    AgentListToggled,
    /// Another agent was picked for the new profile; the list closes.
    AgentPicked(AgentId),
    /// The name changed.
    NameChanged(String),
    /// Another way to give the config folder was picked.
    FolderChoicePicked(FolderChoice),
    /// Something was done in the config folder field: the typed path with
    /// its suggestions, or the path of the new folder, whichever is picked.
    Folder(FolderFieldIntent),
    /// Add or Save was pressed.
    Submitted,
    /// Cancel was pressed, or Esc.
    Cancelled,
}

/// How the config folder of a new profile is given.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FolderChoice {
    /// A path the user types, of a folder that already exists.
    Typed,
    /// A new folder OTTY creates, at the proposed path or one the user
    /// types.
    Create,
}

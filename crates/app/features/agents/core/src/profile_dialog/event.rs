use otty_agents::AgentId;

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
    /// The path of the config folder changed: the typed path or the path
    /// of the new folder, whichever is picked.
    PathChanged(String),
    /// The suggestion at this place of the list under the typed config
    /// folder was pressed.
    SuggestionPicked(usize),
    /// ↓ or ↑ in the typed config folder moved the highlight to this row
    /// of the suggestions, or off the list (`None`).
    SuggestionHighlighted(Option<usize>),
    /// A press away from the suggestions, or Esc while they show: close
    /// them.
    SuggestionsClosed,
    /// Another place of the admin key of an API key profile was picked.
    AdminKeyChoicePicked(AdminKeyChoice),
    /// The path of the admin key file, or the name of its variable,
    /// changed: whichever is picked.
    AdminKeyChanged(String),
    /// The monthly budget of an API key profile changed.
    BudgetChanged(String),
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

/// Where the admin key of an API key profile is kept (spec Decision 9).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdminKeyChoice {
    /// A file that holds the key.
    File,
    /// An environment variable that holds the key.
    Variable,
}

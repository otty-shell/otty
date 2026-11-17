use otty_agents::Profile;

/// What the user does in the New session dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NewSessionDialogIntent {
    /// The text of the folder field changed.
    FolderChanged(String),
    /// The suggestion at this place of the list under the folder field was
    /// pressed.
    SuggestionPicked(usize),
    /// ↓ or ↑ in the folder field moved the highlight to this row of the
    /// suggestions, or off the list (`None`).
    SuggestionHighlighted(Option<usize>),
    /// A press away from the suggestions, or Esc while they show: close
    /// them.
    SuggestionsClosed,
    /// Another profile was picked; the dialog keeps it while OTTY runs.
    ProfilePicked(Profile),
    /// Start, or Enter in the folder field, was pressed.
    Submitted,
    /// Cancel was pressed, or Esc.
    Cancelled,
}

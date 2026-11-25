/// What the user does in a folder field with suggestions: the folder field
/// of the New session dialog, or the config folder field of the profile
/// dialog.
///
/// # Example
///
/// ```
/// use otty_app_agents::{FolderFieldIntent, NewSessionDialogIntent};
///
/// let typed = NewSessionDialogIntent::Folder(FolderFieldIntent::Changed(
///     String::from("~/projects/"),
/// ));
///
/// assert_ne!(
///     typed,
///     NewSessionDialogIntent::Folder(FolderFieldIntent::SuggestionsClosed)
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FolderFieldIntent {
    /// The text of the field changed.
    Changed(String),
    /// The suggestion at this place of the list under the field was
    /// pressed, or picked with Enter or Tab.
    SuggestionPicked(usize),
    /// ↓ or ↑ in the field moved the highlight to this row of the
    /// suggestions, or off the list (`None`).
    SuggestionHighlighted(Option<usize>),
    /// A press away from the suggestions, or Esc while they show: close
    /// them.
    SuggestionsClosed,
}

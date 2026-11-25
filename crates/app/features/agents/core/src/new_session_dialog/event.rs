use otty_agents::Profile;

use crate::folder_field::FolderFieldIntent;

/// What the user does in the New session dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NewSessionDialogIntent {
    /// Something was done in the folder field.
    Folder(FolderFieldIntent),
    /// Another profile was picked; the dialog keeps it while OTTY runs.
    ProfilePicked(Profile),
    /// Start, or Enter in the folder field, was pressed.
    Submitted,
    /// Cancel was pressed, or Esc.
    Cancelled,
}

//! A folder field with suggestions, as the New session dialog and the
//! profile dialog have it (spec Decisions 15 and 17): the typed path, the
//! latest answer of the look-up about it, and whether the folders it may
//! stand for show under the field. `~` stands for the home folder, a path
//! that ends with `/` lists the folders in it, and a pick puts the folder
//! with a `/` in the field, so the next look-up lists the next level. Both
//! dialogs wrap the same intents of the field and draw it the same way.

mod event;
mod full_path;
mod suggestions;
mod view;

pub use event::FolderFieldIntent;
pub(crate) use full_path::{PathProblem, full_path};
pub(crate) use suggestions::FolderSuggestions;
pub(crate) use view::folder_field;

use thiserror::Error;

use super::model::QuickCommandFolder;
use super::state::QuickCommandsState;
use super::storage::QuickCommandsError;

/// Errors returned while validating quick command titles.
#[derive(Debug, Error)]
pub(crate) enum QuickCommandTitleError {
    #[error("Title cannot be empty.")]
    Empty,
    #[error("Title already exists in this folder.")]
    Duplicate,
}

pub(crate) fn normalize_title(
    raw: &str,
    parent: &QuickCommandFolder,
    current: Option<&str>,
) -> Result<String, QuickCommandTitleError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(QuickCommandTitleError::Empty);
    }

    let conflicts = match current {
        Some(existing) => trimmed != existing && parent.contains_title(trimmed),
        None => parent.contains_title(trimmed),
    };
    if conflicts {
        return Err(QuickCommandTitleError::Duplicate);
    }

    Ok(trimmed.to_string())
}

pub(crate) fn persist_dirty(
    state: &mut QuickCommandsState,
) -> Result<(), QuickCommandsError> {
    state.mark_dirty();
    state.persist()
}

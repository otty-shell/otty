use thiserror::Error;

use super::model::QuickLaunchFolder;
use super::state::QuickLaunchesState;
use super::storage::QuickLaunchesError;

/// Errors returned while validating quick launch titles.
#[derive(Debug, Error)]
pub(crate) enum QuickLaunchTitleError {
    #[error("Title cannot be empty.")]
    Empty,
    #[error("Title already exists in this folder.")]
    Duplicate,
}

pub(crate) fn normalize_title(
    raw: &str,
    parent: &QuickLaunchFolder,
    current: Option<&str>,
) -> Result<String, QuickLaunchTitleError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(QuickLaunchTitleError::Empty);
    }

    let conflicts = match current {
        Some(existing) => trimmed != existing && parent.contains_title(trimmed),
        None => parent.contains_title(trimmed),
    };
    if conflicts {
        return Err(QuickLaunchTitleError::Duplicate);
    }

    Ok(trimmed.to_string())
}

pub(crate) fn persist_dirty(
    state: &mut QuickLaunchesState,
) -> Result<(), QuickLaunchesError> {
    state.mark_dirty();
    state.persist()
}

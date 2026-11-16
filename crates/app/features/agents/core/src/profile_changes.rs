//! The new content of `agents.json` after a change of a profile: added,
//! edited, hidden or shown, removed. A removal only drops the entry: the
//! config folder stays on disk.

use std::path::PathBuf;

use otty_agents::{
    AgentDescriptor, DefaultProfileChanges, Profile, SavedProfiles,
};

/// What the profile dialog gives for an edited profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProfileEdit {
    pub(crate) name: String,
    pub(crate) config_dir: PathBuf,
    pub(crate) program: Option<PathBuf>,
}

/// `saved` with the added `profile`.
pub(crate) fn added(saved: &SavedProfiles, profile: Profile) -> SavedProfiles {
    saved.clone().with_added(profile)
}

/// `saved` with `editing`, a profile of the agent of `descriptor`, changed
/// as `edit` says.
///
/// A default profile keeps only what differs from the agent's own: a name
/// other than the agent's name, a folder the user moved it to, a program
/// path. Its folder is written only when the user changed it, so a profile
/// that follows the agent's variable keeps following it. An added profile
/// is replaced whole, in its place.
pub(crate) fn edited(
    saved: &SavedProfiles,
    descriptor: &AgentDescriptor,
    editing: &Profile,
    edit: ProfileEdit,
) -> SavedProfiles {
    let agent = editing.agent();

    if !editing.is_default() {
        let profile = Profile::added(agent, edit.name, edit.config_dir)
            .with_hidden(editing.is_hidden());
        let profile = match edit.program {
            Some(program) => profile.with_program(program),
            None => profile,
        };

        return saved.clone().with_added_replaced(
            agent,
            editing.name(),
            profile,
        );
    }

    let earlier = saved.default_changes_of(agent);
    let is_hidden = earlier.is_some_and(DefaultProfileChanges::is_hidden);
    let mut changes = DefaultProfileChanges::new(agent).with_hidden(is_hidden);

    if edit.name != descriptor.name() {
        changes = changes.with_name(edit.name);
    }
    if edit.config_dir != editing.config_dir() {
        changes = changes.with_config_dir(edit.config_dir);
    } else if let Some(moved) =
        earlier.and_then(DefaultProfileChanges::config_dir)
    {
        changes = changes.with_config_dir(moved.to_path_buf());
    }
    if let Some(program) = edit.program {
        changes = changes.with_program(program);
    }

    saved.clone().with_default_changes(changes)
}

/// `saved` with `profile` hidden or shown. The other changes of a default
/// profile stay.
pub(crate) fn with_hidden(
    saved: &SavedProfiles,
    profile: &Profile,
    is_hidden: bool,
) -> SavedProfiles {
    let agent = profile.agent();

    if !profile.is_default() {
        let changed = profile.clone().with_hidden(is_hidden);
        return saved.clone().with_added_replaced(
            agent,
            profile.name(),
            changed,
        );
    }

    let changes = saved
        .default_changes_of(agent)
        .cloned()
        .unwrap_or_else(|| DefaultProfileChanges::new(agent))
        .with_hidden(is_hidden);

    saved.clone().with_default_changes(changes)
}

/// `saved` without the added `profile`. Nothing else changes, and a
/// default profile can not be removed.
pub(crate) fn removed(
    saved: &SavedProfiles,
    profile: &Profile,
) -> SavedProfiles {
    if profile.is_default() {
        return saved.clone();
    }

    saved.clone().without_added(profile.agent(), profile.name())
}

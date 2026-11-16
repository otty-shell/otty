//! The new content of `agents.json` after a change of a profile: added,
//! edited, removed. A removal only drops the entry: the
//! config folder stays on disk.

use std::path::PathBuf;

use otty_agents::{
    AdminKeyRef, AgentDescriptor, DefaultProfileChanges, Dollars, Profile,
    SavedProfiles,
};

/// What the profile dialog gives for an edited profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProfileEdit {
    pub(crate) name: String,
    pub(crate) config_dir: PathBuf,
    pub(crate) admin_key: Option<AdminKeyRef>,
    pub(crate) monthly_budget: Option<Dollars>,
}

/// `saved` with the added `profile`.
pub(crate) fn added(saved: &SavedProfiles, profile: Profile) -> SavedProfiles {
    saved.clone().with_added(profile)
}

/// `saved` with `editing`, a profile of the agent of `descriptor`, changed
/// as `edit` says.
///
/// A default profile keeps only what differs from the agent's own: a name
/// other than the agent's name, a folder the user moved it to, the admin
/// key and the budget. Its folder is written only when the user changed
/// it, so a profile that follows the agent's variable keeps following it.
/// An added profile is replaced whole, in its place.
pub(crate) fn edited(
    saved: &SavedProfiles,
    descriptor: &AgentDescriptor,
    editing: &Profile,
    edit: ProfileEdit,
) -> SavedProfiles {
    let agent = editing.agent();

    if !editing.is_default() {
        let profile = Profile::added(agent, edit.name, edit.config_dir)
            .with_admin_key(edit.admin_key)
            .with_monthly_budget(edit.monthly_budget);

        return saved.clone().with_added_replaced(
            agent,
            editing.name(),
            profile,
        );
    }

    let earlier = saved.default_changes_of(agent);
    let mut changes = DefaultProfileChanges::new(agent)
        .with_admin_key(edit.admin_key)
        .with_monthly_budget(edit.monthly_budget);

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

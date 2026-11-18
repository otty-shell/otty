//! The new content of `agents.json` after a change of a profile: edited or
//! removed. A removal only drops the entry: the config folder stays on
//! disk.

use std::path::PathBuf;

use otty_agents::{AdminKeyRef, Dollars, Profile, SavedProfiles};

/// What the profile dialog gives for an edited profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProfileEdit {
    pub(crate) name: String,
    pub(crate) config_dir: PathBuf,
    pub(crate) admin_key: Option<AdminKeyRef>,
    pub(crate) monthly_budget: Option<Dollars>,
}

/// `saved` with `editing`, a profile of the agent whose own name is
/// `agent_name`, changed as `edit` says. A default profile keeps only what
/// differs from the agent's own ([`SavedProfiles::with_default_edited`]);
/// an added profile is replaced whole, in its place.
pub(crate) fn edited(
    saved: &SavedProfiles,
    agent_name: &str,
    editing: &Profile,
    edit: ProfileEdit,
) -> SavedProfiles {
    let agent = editing.agent();

    if editing.is_default() {
        let profile = Profile::default_for(agent, edit.name, edit.config_dir)
            .with_admin_key(edit.admin_key)
            .with_monthly_budget(edit.monthly_budget);

        return saved
            .clone()
            .with_default_edited(agent_name, editing, profile);
    }

    let profile = Profile::added(agent, edit.name, edit.config_dir)
        .with_admin_key(edit.admin_key)
        .with_monthly_budget(edit.monthly_budget);
    saved
        .clone()
        .with_added_replaced(agent, editing.name(), profile)
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

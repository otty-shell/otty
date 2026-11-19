//! The new content of `agents.json` after a change of a profile: added,
//! edited or removed. A removal only drops the entry: the config folder
//! stays on disk.

use std::path::PathBuf;

use otty_agents::{Account, AdminKeyRef, AgentId, Profile, SavedProfiles};

/// What the profile dialog gives for an edited profile: its name, its
/// config folder, and the admin key of its organization when the dialog
/// had that field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProfileEdit {
    pub(crate) name: String,
    pub(crate) config_dir: PathBuf,
    pub(crate) admin_key: Option<AdminKeyEdit>,
}

/// The admin key of an organization as the profile dialog left it: where
/// it is kept, or none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AdminKeyEdit {
    pub(crate) organization: Account,
    pub(crate) admin_key: Option<AdminKeyRef>,
}

/// `saved` with one more profile of `agent` the user added, named `name`,
/// in `config_dir`.
pub(crate) fn added(
    saved: &SavedProfiles,
    agent: AgentId,
    name: String,
    config_dir: PathBuf,
) -> SavedProfiles {
    let profile = Profile::added(agent, name, config_dir);

    saved.clone().with_added(profile)
}

/// `saved` with `editing`, a profile of the agent whose own name is
/// `agent_name`, changed as `edit` says. A default profile keeps only what
/// differs from the agent's own ([`SavedProfiles::with_default_edited`]);
/// an added profile is replaced whole, in its place. The admin key is
/// saved for the organization, so every profile of it shares it.
pub(crate) fn edited(
    saved: &SavedProfiles,
    agent_name: &str,
    editing: &Profile,
    edit: ProfileEdit,
) -> SavedProfiles {
    let agent = editing.agent();
    let mut saved = saved.clone();

    if let Some(key) = edit.admin_key {
        saved = saved.with_admin_key_of(&key.organization, key.admin_key);
    }

    if editing.is_default() {
        let profile = Profile::default_for(agent, edit.name, edit.config_dir);
        return saved.with_default_edited(agent_name, editing, profile);
    }

    let profile = Profile::added(agent, edit.name, edit.config_dir);
    saved.with_added_replaced(agent, editing.name(), profile)
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

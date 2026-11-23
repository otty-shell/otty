//! The text of `agents.json` as it is read and written, and how it turns
//! into [`SavedProfiles`] and back.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::hidden_profiles::HiddenProfile;
use super::own_logins::is_plain_id;
use super::saved_accounts::SavedAccount;
use super::{DefaultProfileChanges, SavedProfiles};
use crate::account::AccountKey;
use crate::agent::AgentId;
use crate::given_key::{GivenKey, KeyRef};
use crate::own_login::OwnLogin;
use crate::profile::Profile;
use crate::provider::ProviderId;

/// The whole text of `agents.json`.
#[derive(Debug, Default, Serialize, Deserialize)]
pub(super) struct FileContent {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    default_profiles: BTreeMap<String, DefaultEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    added_profiles: Vec<AddedEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    accounts: Vec<AccountEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    hidden_profiles: Vec<HiddenProfileEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    own_logins: Vec<OwnLoginEntry>,
    #[serde(default, skip_serializing_if = "is_false")]
    found_report_shown: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    keyring_allowed: bool,
}

impl FileContent {
    /// The text of `saved`.
    pub(super) fn of_saved(saved: &SavedProfiles) -> Self {
        let default_profiles: BTreeMap<String, DefaultEntry> = saved
            .default_changes
            .iter()
            .map(|changes| {
                let agent = String::from(changes.agent.as_str());
                (agent, DefaultEntry::from_changes(changes))
            })
            .collect();

        let added_profiles =
            saved.added.iter().map(AddedEntry::from_profile).collect();

        Self {
            default_profiles,
            added_profiles,
            accounts: saved
                .accounts
                .iter()
                .map(AccountEntry::from_saved)
                .collect(),
            hidden_profiles: saved
                .hidden_profiles
                .iter()
                .map(HiddenProfileEntry::from_hidden)
                .collect(),
            own_logins: saved
                .own_logins
                .iter()
                .map(OwnLoginEntry::from_login)
                .collect(),
            found_report_shown: saved.is_found_report_shown,
            keyring_allowed: saved.is_keyring_allowed,
        }
    }

    /// The content this text holds. The entries of every agent and
    /// plugin are kept, whether or not an entry of the registry names
    /// them; an account entry that names no plugin or no id, or keeps
    /// nothing, a given key of no place and an own login whose id is no
    /// plain name are left out.
    pub(super) fn into_saved(self) -> SavedProfiles {
        let mut saved = SavedProfiles::default();

        for (agent, entry) in self.default_profiles {
            saved = saved.with_default_changes(DefaultProfileChanges {
                agent: AgentId::new(agent),
                name: entry.name,
                config_dir: entry.config_dir,
            });
        }

        for entry in self.added_profiles {
            saved = saved.with_added(Profile::added(
                AgentId::new(entry.agent),
                entry.name,
                entry.config_dir,
            ));
        }

        saved.accounts = self
            .accounts
            .into_iter()
            .filter_map(AccountEntry::into_saved)
            .collect();
        saved.hidden_profiles = self
            .hidden_profiles
            .into_iter()
            .map(HiddenProfileEntry::into_hidden)
            .collect();
        saved.own_logins = self
            .own_logins
            .into_iter()
            .filter_map(OwnLoginEntry::into_login)
            .collect();
        saved.is_found_report_shown = self.found_report_shown;
        saved.is_keyring_allowed = self.keyring_allowed;

        saved
    }
}

/// The changes to one default profile in the text.
#[derive(Debug, Default, Serialize, Deserialize)]
struct DefaultEntry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    config_dir: Option<PathBuf>,
}

impl DefaultEntry {
    fn from_changes(changes: &DefaultProfileChanges) -> Self {
        Self {
            name: changes.name.clone(),
            config_dir: changes.config_dir.clone(),
        }
    }
}

/// One added profile in the text.
#[derive(Debug, Serialize, Deserialize)]
struct AddedEntry {
    agent: String,
    name: String,
    config_dir: PathBuf,
}

impl AddedEntry {
    fn from_profile(profile: &Profile) -> Self {
        Self {
            agent: String::from(profile.agent().as_str()),
            name: String::from(profile.name()),
            config_dir: profile.config_dir().to_path_buf(),
        }
    }
}

/// One account in the text, by the plugin whose provider part found it
/// and the part's id of it, with the user's name for it, whether it is
/// hidden and where the keys the user gave for it are kept, each under
/// its slot.
#[derive(Debug, Serialize, Deserialize)]
struct AccountEntry {
    #[serde(default)]
    plugin: String,
    #[serde(default)]
    id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    hidden: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    given_keys: Vec<GivenKeyEntry>,
}

impl AccountEntry {
    fn from_saved(saved: &SavedAccount) -> Self {
        Self {
            plugin: saved.key.plugin().to_string(),
            id: saved.key.id().to_string(),
            name: saved.name.clone(),
            hidden: saved.is_hidden,
            given_keys: saved
                .given_keys
                .iter()
                .map(GivenKeyEntry::from_given)
                .collect(),
        }
    }

    /// The choices for the account; `None` for an entry that names no
    /// plugin or no id, or one that keeps nothing. A plugin no entry of
    /// the registry names is kept as it is.
    fn into_saved(self) -> Option<SavedAccount> {
        if self.plugin.is_empty() || self.id.is_empty() {
            return None;
        }

        let given_keys = self
            .given_keys
            .into_iter()
            .filter_map(GivenKeyEntry::into_given)
            .collect();
        let saved = SavedAccount {
            key: AccountKey::new(self.plugin, self.id),
            name: self.name,
            is_hidden: self.hidden,
            given_keys,
        };
        (!saved.is_empty()).then_some(saved)
    }
}

/// One hidden profile in the text: its agent and its config folder.
#[derive(Debug, Serialize, Deserialize)]
struct HiddenProfileEntry {
    agent: String,
    config_dir: PathBuf,
}

impl HiddenProfileEntry {
    fn from_hidden(hidden: &HiddenProfile) -> Self {
        Self {
            agent: String::from(hidden.agent.as_str()),
            config_dir: hidden.config_dir.clone(),
        }
    }

    /// The hidden profile, of any agent.
    fn into_hidden(self) -> HiddenProfile {
        HiddenProfile {
            agent: AgentId::new(self.agent),
            config_dir: self.config_dir,
        }
    }
}

/// One own login in the text: its id, the plugin, the provider part and
/// the agent part. Its folder is not written.
#[derive(Debug, Serialize, Deserialize)]
struct OwnLoginEntry {
    #[serde(default)]
    id: String,
    #[serde(default)]
    plugin: String,
    #[serde(default)]
    provider: String,
    #[serde(default)]
    agent: String,
}

impl OwnLoginEntry {
    fn from_login(login: &OwnLogin) -> Self {
        Self {
            id: login.id().to_string(),
            plugin: login.plugin().to_string(),
            provider: login.provider().as_str().to_string(),
            agent: login.agent().as_str().to_string(),
        }
    }

    /// The own login, of any plugin; `None` when its id is no plain name,
    /// so its folder would be another place than `accounts/<id>`, or when
    /// it names no plugin, provider part or agent part.
    fn into_login(self) -> Option<OwnLogin> {
        let names_its_parts = !self.plugin.is_empty()
            && !self.provider.is_empty()
            && !self.agent.is_empty();
        if !is_plain_id(&self.id) || !names_its_parts {
            return None;
        }

        Some(OwnLogin::new(
            self.id,
            self.plugin,
            ProviderId::new(self.provider),
            AgentId::new(self.agent),
        ))
    }
}

/// A key the user gave, in the text: the slot of the part that asked for
/// it and where it is kept. Only the path or the name is written, never
/// the key.
#[derive(Debug, Serialize, Deserialize)]
struct GivenKeyEntry {
    slot: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    file: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    env_var: Option<String>,
}

impl GivenKeyEntry {
    fn from_given(given: &GivenKey) -> Self {
        let (file, env_var) = match given.key() {
            KeyRef::File(path) => (Some(path.clone()), None),
            KeyRef::EnvVar(name) => (None, Some(name.clone())),
        };

        Self {
            slot: given.slot().to_string(),
            file,
            env_var,
        }
    }

    /// The given key; `None` with no slot or no place. The file wins when
    /// both are written.
    fn into_given(self) -> Option<GivenKey> {
        let slot = self.slot.trim();
        if slot.is_empty() {
            return None;
        }

        let key = match (self.file, self.env_var) {
            (Some(path), _) => KeyRef::File(path),
            (None, Some(name)) => KeyRef::EnvVar(name),
            (None, None) => return None,
        };
        Some(GivenKey::new(slot, key))
    }
}

/// Whether `value` is false: a flag that is off is not written.
fn is_false(value: &bool) -> bool {
    !value
}

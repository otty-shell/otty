//! The text of `agents.json` as it is read and written, and how it turns
//! into [`SavedProfiles`] and back.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::admin_keys::{OldAdminKey, OldKeyOwner};
use super::hidden_profiles::HiddenProfile;
use super::saved_accounts::SavedAccount;
use super::{DefaultProfileChanges, SavedProfiles};
use crate::account::AccountIdentity;
use crate::admin_key::AdminKeyRef;
use crate::agent::{Agent, AgentId};
use crate::profile::Profile;
use crate::provider::Provider;

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
    #[serde(default, skip_serializing_if = "is_false")]
    found_report_shown: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    keyring_allowed: bool,
}

impl FileContent {
    /// The text of `saved`, with each admin key not yet moved in the entry
    /// of its profile.
    pub(super) fn of_saved(saved: &SavedProfiles) -> Self {
        let mut default_profiles: BTreeMap<String, DefaultEntry> = saved
            .default_changes
            .iter()
            .map(|changes| {
                let agent = String::from(changes.agent.as_str());
                (agent, DefaultEntry::from_changes(changes))
            })
            .collect();
        for old in &saved.old_admin_keys {
            if old.owner == OldKeyOwner::DefaultProfile {
                let agent = String::from(old.agent.as_str());
                default_profiles.entry(agent).or_default().admin_key =
                    Some(AdminKeyEntry::from_ref(&old.admin_key));
            }
        }

        let added_profiles = saved
            .added
            .iter()
            .map(|profile| {
                let old_key = saved
                    .old_admin_keys
                    .iter()
                    .find(|old| {
                        old.is_of_added(profile.agent(), profile.name())
                    })
                    .map(|old| &old.admin_key);
                AddedEntry::from_profile(profile, old_key)
            })
            .collect();

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
            found_report_shown: saved.is_found_report_shown,
            keyring_allowed: saved.is_keyring_allowed,
        }
    }

    /// The content this text holds. The entries of an agent not in
    /// `agents`, of a provider this version does not know, an account entry
    /// that names no account or keeps nothing, and an admin key of no kind
    /// are left out.
    pub(super) fn into_saved(self, agents: &[Box<dyn Agent>]) -> SavedProfiles {
        let mut saved = SavedProfiles::default();

        for (agent, entry) in self.default_profiles {
            let Some(agent) = known_agent_id(&agent, agents) else {
                continue;
            };

            if let Some(admin_key) =
                entry.admin_key.and_then(AdminKeyEntry::into_ref)
            {
                saved.old_admin_keys.push(OldAdminKey {
                    agent,
                    owner: OldKeyOwner::DefaultProfile,
                    admin_key,
                });
            }
            saved = saved.with_default_changes(DefaultProfileChanges {
                agent,
                name: entry.name,
                config_dir: entry.config_dir,
            });
        }

        for entry in self.added_profiles {
            let Some(agent) = known_agent_id(&entry.agent, agents) else {
                continue;
            };

            if let Some(admin_key) =
                entry.admin_key.and_then(AdminKeyEntry::into_ref)
            {
                saved.old_admin_keys.push(OldAdminKey {
                    agent,
                    owner: OldKeyOwner::AddedProfile(entry.name.clone()),
                    admin_key,
                });
            }
            saved = saved.with_added(Profile::added(
                agent,
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
            .filter_map(|entry| entry.into_hidden(agents))
            .collect();
        saved.is_found_report_shown = self.found_report_shown;
        saved.is_keyring_allowed = self.keyring_allowed;

        saved
    }
}

/// The changes to one default profile in the text. An admin key here is
/// one an older version kept in the profile.
#[derive(Debug, Default, Serialize, Deserialize)]
struct DefaultEntry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    config_dir: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    admin_key: Option<AdminKeyEntry>,
}

impl DefaultEntry {
    fn from_changes(changes: &DefaultProfileChanges) -> Self {
        Self {
            name: changes.name.clone(),
            config_dir: changes.config_dir.clone(),
            admin_key: None,
        }
    }
}

/// One added profile in the text. An admin key here is one an older
/// version kept in the profile.
#[derive(Debug, Serialize, Deserialize)]
struct AddedEntry {
    agent: String,
    name: String,
    config_dir: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    admin_key: Option<AdminKeyEntry>,
}

impl AddedEntry {
    fn from_profile(profile: &Profile, old_key: Option<&AdminKeyRef>) -> Self {
        Self {
            agent: String::from(profile.agent().as_str()),
            name: String::from(profile.name()),
            config_dir: profile.config_dir().to_path_buf(),
            admin_key: old_key.map(AdminKeyEntry::from_ref),
        }
    }
}

/// One account in the text, by its provider and one of its account id,
/// organization id or email, with the user's name for it, whether it is
/// hidden and, for an organization, where its admin key is kept.
#[derive(Debug, Serialize, Deserialize)]
struct AccountEntry {
    provider: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    account_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    organization_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    email: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    hidden: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    admin_key: Option<AdminKeyEntry>,
}

impl AccountEntry {
    fn from_saved(saved: &SavedAccount) -> Self {
        let mut entry = Self {
            provider: String::from(saved.provider.id()),
            account_id: None,
            organization_id: None,
            email: None,
            name: saved.name.clone(),
            hidden: saved.is_hidden,
            admin_key: saved.admin_key.as_ref().map(AdminKeyEntry::from_ref),
        };

        match &saved.identity {
            AccountIdentity::AccountId(id) => {
                entry.account_id = Some(id.clone())
            },
            AccountIdentity::OrganizationId(id) => {
                entry.organization_id = Some(id.clone());
            },
            AccountIdentity::Email(email) => entry.email = Some(email.clone()),
            // A key is never saved (`SavedAccount::new`).
            AccountIdentity::KeyFingerprint(_) => {},
        }
        entry
    }

    /// The choices for the account; `None` for a provider this version
    /// does not know, an entry that names no account, or one that keeps
    /// nothing. An admin key is kept for an organization only.
    fn into_saved(self) -> Option<SavedAccount> {
        let provider = Provider::of_id(&self.provider)?;
        let identity = match (self.account_id, self.organization_id, self.email)
        {
            (Some(id), _, _) => AccountIdentity::AccountId(id),
            (None, Some(id), _) => AccountIdentity::OrganizationId(id),
            (None, None, Some(email)) => AccountIdentity::Email(email),
            (None, None, None) => return None,
        };
        let is_organization =
            matches!(identity, AccountIdentity::OrganizationId(_));
        let admin_key = self
            .admin_key
            .and_then(AdminKeyEntry::into_ref)
            .filter(|_| is_organization);

        let saved = SavedAccount {
            provider,
            identity,
            name: self.name,
            is_hidden: self.hidden,
            admin_key,
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

    /// The hidden profile; `None` for an agent not in `agents`.
    fn into_hidden(self, agents: &[Box<dyn Agent>]) -> Option<HiddenProfile> {
        let agent = known_agent_id(&self.agent, agents)?;

        Some(HiddenProfile {
            agent,
            config_dir: self.config_dir,
        })
    }
}

/// Where the admin key is kept, in the text: a file or a variable. Only
/// the path or the name is written, never the key.
#[derive(Debug, Serialize, Deserialize)]
struct AdminKeyEntry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    file: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    env_var: Option<String>,
}

impl AdminKeyEntry {
    fn from_ref(reference: &AdminKeyRef) -> Self {
        match reference {
            AdminKeyRef::File(path) => Self {
                file: Some(path.clone()),
                env_var: None,
            },
            AdminKeyRef::EnvVar(name) => Self {
                file: None,
                env_var: Some(name.clone()),
            },
        }
    }

    /// The place of the key; the file when both are written, none when
    /// neither is.
    fn into_ref(self) -> Option<AdminKeyRef> {
        match (self.file, self.env_var) {
            (Some(path), _) => Some(AdminKeyRef::File(path)),
            (None, Some(name)) => Some(AdminKeyRef::EnvVar(name)),
            (None, None) => None,
        }
    }
}

/// Whether `value` is false: a flag that is off is not written.
fn is_false(value: &bool) -> bool {
    !value
}

/// The id of the agent of `agents` saved as `id`, if it is one of them.
fn known_agent_id(id: &str, agents: &[Box<dyn Agent>]) -> Option<AgentId> {
    agents
        .iter()
        .map(|agent| agent.descriptor().id())
        .find(|known| known.as_str() == id)
}

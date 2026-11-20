//! The text of `agents.json` as it is read and written, and how it turns
//! into [`SavedProfiles`] and back.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::hidden_profiles::HiddenProfile;
use super::saved_accounts::SavedAccount;
use super::{DefaultProfileChanges, SavedProfiles};
use crate::account::AccountIdentity;
use crate::admin_key::AdminKeyRef;
use crate::agent::AgentId;
use crate::profile::Profile;
use crate::provider::ProviderRef;

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
            found_report_shown: saved.is_found_report_shown,
            keyring_allowed: saved.is_keyring_allowed,
        }
    }

    /// The content this text holds. The entries of every agent and
    /// provider are kept, whether or not an entry of the registry names
    /// them; an account entry that names no provider or no account, or
    /// keeps nothing, and an admin key of no kind are left out.
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

/// One account in the text, by its provider and one of its account id,
/// organization id, email or, for an account the user added, the place of
/// its key, with the user's name for it, whether it is hidden and, for an
/// organization, where its admin key is kept.
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
    key: Option<AdminKeyEntry>,
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
            provider: saved.provider.to_saved(),
            account_id: None,
            organization_id: None,
            email: None,
            key: None,
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
            AccountIdentity::AddedKey(key) => {
                entry.key = Some(AdminKeyEntry::from_ref(key));
            },
            // A key OTTY found is never saved (`SavedAccount::new`).
            AccountIdentity::KeyFingerprint(_) => {},
        }
        entry
    }

    /// The choices for the account; `None` for an entry that names no
    /// provider or no account, or one that keeps nothing. A provider no
    /// entry of the registry names is kept as it is. An admin key is kept
    /// for an organization only.
    fn into_saved(self) -> Option<SavedAccount> {
        let provider = ProviderRef::from_saved(&self.provider)?;
        let added_key = self.key.and_then(AdminKeyEntry::into_ref);
        let identity = match (self.account_id, self.organization_id, self.email)
        {
            (Some(id), _, _) => AccountIdentity::AccountId(id),
            (None, Some(id), _) => AccountIdentity::OrganizationId(id),
            (None, None, Some(email)) => AccountIdentity::Email(email),
            (None, None, None) => AccountIdentity::AddedKey(added_key?),
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

    /// The hidden profile, of any agent.
    fn into_hidden(self) -> HiddenProfile {
        HiddenProfile {
            agent: AgentId::new(self.agent),
            config_dir: self.config_dir,
        }
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

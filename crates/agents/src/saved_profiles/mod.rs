//! The content of `agents.json`: the profiles the user added, what they
//! changed in the default profiles, the user's names and hidden accounts,
//! the hidden profiles, and where the admin keys of the organizations are
//! kept (spec Decisions 1 and 7).
//!
//! The file holds names and paths only, never a secret. Reading and writing
//! the file itself is the app's job; this module turns its content to and
//! from text. A default profile is listed under its agent's id with only
//! the parts the user changed; an added profile is a whole entry; an
//! account is listed by its provider and its account id, organization id
//! or email; a hidden profile by its agent and its config folder:
//!
//! ```json
//! {
//!   "default_profiles": {
//!     "claude-code": {
//!       "name": "Claude Code · Home",
//!       "config_dir": "/data/claude"
//!     },
//!     "codex": { "name": "Codex · Home" }
//!   },
//!   "added_profiles": [
//!     {
//!       "agent": "claude-code",
//!       "name": "Claude Code · Semrush",
//!       "config_dir": "/home/user/.claude-semrush"
//!     }
//!   ],
//!   "accounts": [
//!     {
//!       "provider": "anthropic",
//!       "organization_id": "00000000-0000-4000-8000-00000000000b",
//!       "name": "Acme",
//!       "admin_key": { "file": "/home/user/.config/anthropic-admin-key" }
//!     },
//!     { "provider": "openai", "account_id": "made-up-account", "hidden": true }
//!   ],
//!   "hidden_profiles": [
//!     { "agent": "codex", "config_dir": "/home/user/.codex-old" }
//!   ],
//!   "found_report_shown": true,
//!   "keyring_allowed": true
//! }
//! ```
//!
//! `found_report_shown` tells that the Found by OTTY report was shown once,
//! so it does not open by itself again. `keyring_allowed` tells that the
//! user allowed OTTY to read the entries of the known agents in the system
//! keyring. A profile OTTY found is not
//! written; once the user changes it, it is saved as an added profile. It
//! can still be hidden: a hidden profile is kept by its folder, not by an
//! entry.
//!
//! An organization paid per token may keep where its admin key is, a file
//! or an environment variable; the key itself is never written. An account
//! the user added is written by its provider and the place of its key,
//! `"key": { "file": … }` or `"key": { "env_var": … }`. A found account is
//! written by its account id, organization id or email only, never by
//! anything taken from a key: an account known only by its key gets no
//! name and can not be hidden in the file.
//!
//! Older versions kept the admin key in the entry of a profile. Such a key
//! is moved to the organization of the profile by
//! [`SavedProfiles::with_admin_keys_moved`] once the accounts of the
//! profiles are known, and stays in its entry until then.
//!
//! Every part may be missing. The entries of an agent or a provider no
//! entry of the registry names are kept as they are and not read, so they
//! come back when the entry does (spec 2026-11-19-agent-plugins,
//! "`agents.json`"). An account of a provider no entry serves may be
//! written by the host of its server, `"provider": "host:api.deepseek.com"`.
//! An unknown field is skipped. So is what older versions wrote and this
//! one no longer keeps: `hidden` in a profile entry (hidden profiles are
//! listed apart now), `program` (the program is found by its name) and
//! `monthly_budget` (OTTY only shows, spec Decision 5).

mod admin_keys;
mod file_content;
mod hidden_profiles;
mod saved_accounts;

use std::path::{Path, PathBuf};

use self::admin_keys::{OldAdminKey, OldKeyOwner};
use self::file_content::FileContent;
use self::hidden_profiles::HiddenProfile;
use self::saved_accounts::SavedAccount;
use crate::account::Account;
use crate::admin_key::AdminKeyRef;
use crate::agent::AgentId;
use crate::profile::Profile;
use crate::provider::ProviderRef;
use crate::registry::Registry;

/// The content of `agents.json`: the profiles the user added, what they
/// changed in the default profiles, the user's names and hidden accounts,
/// the hidden profiles, and where the admin keys of the organizations are
/// kept.
///
/// ```
/// use std::path::PathBuf;
///
/// use otty_agents::{AgentId, Profile, SavedProfiles};
///
/// let saved = SavedProfiles::default().with_added(Profile::added(
///     AgentId::new("codex"),
///     "Codex · Work",
///     PathBuf::from("/home/user/.codex-work"),
/// ));
///
/// let text = saved.to_json().expect("the paths are text");
/// let loaded = SavedProfiles::from_json(&text);
///
/// assert_eq!(loaded.expect("the text was just written"), saved);
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SavedProfiles {
    default_changes: Vec<DefaultProfileChanges>,
    added: Vec<Profile>,
    accounts: Vec<SavedAccount>,
    old_admin_keys: Vec<OldAdminKey>,
    hidden_profiles: Vec<HiddenProfile>,
    is_found_report_shown: bool,
    is_keyring_allowed: bool,
}

impl SavedProfiles {
    /// What the user changed in the default profile of `agent`, if
    /// anything.
    pub fn default_changes_of(
        &self,
        agent: &AgentId,
    ) -> Option<&DefaultProfileChanges> {
        self.default_changes
            .iter()
            .find(|changes| changes.agent == *agent)
    }

    /// The profiles the user added, in the order they were added.
    pub fn added(&self) -> &[Profile] {
        &self.added
    }

    /// Where the admin key of `account` is kept, when the user gave one;
    /// only an organization has one (spec Decision 7).
    pub fn admin_key_of(&self, account: &Account) -> Option<&AdminKeyRef> {
        self.saved_account(account)?.admin_key.as_ref()
    }

    /// The accounts the user added by hand, in the order they were added
    /// (spec 2026-11-19, Decision 7).
    pub fn added_accounts(&self) -> Vec<Account> {
        self.accounts
            .iter()
            .filter_map(SavedAccount::added_account)
            .collect()
    }

    /// The name the user gave `account`, if any (spec 2026-11-19,
    /// Decision 7).
    pub fn account_name(&self, account: &Account) -> Option<&str> {
        self.saved_account(account)?.name.as_deref()
    }

    /// Whether the user hid `account`: OTTY does not read it (spec
    /// 2026-11-19, Decision 7).
    pub fn is_account_hidden(&self, account: &Account) -> bool {
        self.saved_account(account)
            .is_some_and(|saved| saved.is_hidden)
    }

    /// Whether the user hid `profile`, by its agent and its config folder:
    /// OTTY does not read it (spec 2026-11-19, Decision 7).
    pub fn is_profile_hidden(&self, profile: &Profile) -> bool {
        self.hidden_profiles
            .iter()
            .any(|hidden| hidden.is_of(profile))
    }

    /// Whether the Found by OTTY report was shown once: it opens by itself
    /// on the first start only (spec Decision 2).
    pub fn is_found_report_shown(&self) -> bool {
        self.is_found_report_shown
    }

    /// Whether the user allowed OTTY to read the entries of the known
    /// agents in the system keyring (spec 2026-11-19, Decision 8).
    pub fn is_keyring_allowed(&self) -> bool {
        self.is_keyring_allowed
    }

    /// The content read from the text of `agents.json`.
    ///
    /// An empty text is no changes. The entries of every agent and
    /// provider are kept, whether or not an entry of the registry names
    /// them, and unknown fields are skipped. Only text that is not this
    /// layout at all is an error.
    pub fn from_json(text: &str) -> Result<Self, SavedProfilesError> {
        if text.trim().is_empty() {
            return Ok(Self::default());
        }

        let file: FileContent =
            serde_json::from_str(text).map_err(SavedProfilesError::Read)?;

        Ok(file.into_saved())
    }

    /// The text of `agents.json`.
    ///
    /// It fails only when a path is not valid UTF-8 text.
    pub fn to_json(&self) -> Result<String, SavedProfilesError> {
        let file = FileContent::of_saved(self);

        let mut text = serde_json::to_string_pretty(&file)
            .map_err(SavedProfilesError::Write)?;
        text.push('\n');

        Ok(text)
    }

    /// The same content with `changes` to the default profile of their
    /// agent, in place of the earlier changes of that agent.
    pub fn with_default_changes(
        mut self,
        changes: DefaultProfileChanges,
    ) -> Self {
        self.default_changes
            .retain(|earlier| earlier.agent != changes.agent);
        self.default_changes.push(changes);
        self
    }

    /// The same content with the default profile `editing`, of the agent
    /// whose own name is `agent_name`, saved as the user left it,
    /// `edited`: its name and config folder.
    ///
    /// Only what differs from the agent's own is kept: a name other than
    /// the agent's name, and the folder only when the user changed it, so
    /// a profile that follows the agent's variable keeps following it; a
    /// folder the user moved it to before stays.
    ///
    /// ```
    /// use std::path::PathBuf;
    ///
    /// use otty_agents::{AgentId, Profile, SavedProfiles};
    ///
    /// let codex = Profile::default_for(
    ///     AgentId::new("codex"),
    ///     "Codex",
    ///     PathBuf::from("/home/user/.codex"),
    /// );
    /// let renamed = Profile::default_for(
    ///     AgentId::new("codex"),
    ///     "Codex · Home",
    ///     PathBuf::from("/home/user/.codex"),
    /// );
    ///
    /// let saved =
    ///     SavedProfiles::default().with_default_edited("Codex", &codex, renamed);
    /// let changes = saved.default_changes_of(&AgentId::new("codex"));
    ///
    /// assert_eq!(
    ///     changes.and_then(|changes| changes.name()),
    ///     Some("Codex · Home")
    /// );
    /// assert_eq!(changes.and_then(|changes| changes.config_dir()), None);
    /// ```
    pub fn with_default_edited(
        self,
        agent_name: &str,
        editing: &Profile,
        edited: Profile,
    ) -> Self {
        let agent = editing.agent();
        let moved_before = self
            .default_changes_of(agent)
            .and_then(DefaultProfileChanges::config_dir)
            .map(Path::to_path_buf);

        let mut changes = DefaultProfileChanges::new(agent.clone());
        if edited.name() != agent_name {
            changes = changes.with_name(edited.name());
        }
        if edited.config_dir() != editing.config_dir() {
            changes =
                changes.with_config_dir(edited.config_dir().to_path_buf());
        } else if let Some(moved) = moved_before {
            changes = changes.with_config_dir(moved);
        }

        self.with_default_changes(changes)
    }

    /// The same content, remembering that the Found by OTTY report was
    /// shown.
    pub fn with_found_report_shown(mut self) -> Self {
        self.is_found_report_shown = true;
        self
    }

    /// The same content, remembering that the user allowed OTTY to read
    /// the system keyring.
    pub fn with_keyring_allowed(mut self) -> Self {
        self.is_keyring_allowed = true;
        self
    }

    /// The same content with one more profile the user added.
    pub fn with_added(mut self, profile: Profile) -> Self {
        self.added.push(profile);
        self
    }

    /// The same content with the added profile of `agent` named `name`
    /// replaced by `profile`, in its place. A profile that is not saved
    /// is added at the end. An admin key an older version kept in its
    /// entry stays with it.
    ///
    /// Names are unique ([`check_profile_name`](crate::check_profile_name)),
    /// so the agent and the name find one profile.
    pub fn with_added_replaced(
        mut self,
        agent: &AgentId,
        name: &str,
        profile: Profile,
    ) -> Self {
        let position = self
            .added
            .iter()
            .position(|saved| saved.agent() == agent && saved.name() == name);

        for old in &mut self.old_admin_keys {
            if old.is_of_added(agent, name) {
                old.owner = OldKeyOwner::AddedProfile(profile.name().into());
            }
        }

        match position {
            Some(position) => self.added[position] = profile,
            None => self.added.push(profile),
        }

        self
    }

    /// The same content without the added profile of `agent` named `name`.
    /// Only the entry goes: its config folder stays where it is.
    pub fn without_added(mut self, agent: &AgentId, name: &str) -> Self {
        self.added
            .retain(|saved| !(saved.agent() == agent && saved.name() == name));
        self.old_admin_keys
            .retain(|old| !old.is_of_added(agent, name));
        self
    }

    /// The same content with `admin_key` as the place of the admin key of
    /// `account`, or with no admin key for it. Only an organization takes
    /// one ([`Account::organization_id`]): for another account the
    /// content stays as it is, so nothing taken from a key is ever saved.
    ///
    /// ```
    /// use std::path::PathBuf;
    ///
    /// use otty_agents::{
    ///     Account, AccountIdentity, AccountKind, AccountLogin, AdminKeyRef,
    ///     FoundIn, ProviderRef, SavedProfiles,
    /// };
    ///
    /// let organization = Account::new(
    ///     ProviderRef::of_id("anthropic"),
    ///     AccountIdentity::OrganizationId("made-up-org".into()),
    ///     AccountLogin::Email("dev@example.com".into()),
    ///     AccountKind::Money,
    ///     FoundIn::ProfileFolder(PathBuf::from("/home/user/.claude")),
    /// );
    /// let key = AdminKeyRef::EnvVar("ANTHROPIC_ADMIN_KEY".into());
    ///
    /// let saved = SavedProfiles::default()
    ///     .with_admin_key_of(&organization, Some(key.clone()));
    ///
    /// assert_eq!(saved.admin_key_of(&organization), Some(&key));
    /// ```
    pub fn with_admin_key_of(
        self,
        account: &Account,
        admin_key: Option<AdminKeyRef>,
    ) -> Self {
        if account.organization_id().is_none() {
            return self;
        }

        self.with_account_changed(account, |saved| saved.admin_key = admin_key)
    }

    /// The same content with `name` as the user's name of `account`, or
    /// with none, so it is shown by its provider and login again. Only an
    /// account that can be saved ([`Account::can_be_saved`]) keeps one: for
    /// a key the content stays as it is.
    ///
    /// ```
    /// use std::path::PathBuf;
    ///
    /// use otty_agents::{
    ///     Account, AccountIdentity, AccountKind, AccountLogin, FoundIn,
    ///     ProviderRef, SavedProfiles,
    /// };
    ///
    /// let login = Account::new(
    ///     ProviderRef::of_id("openai"),
    ///     AccountIdentity::AccountId("made-up-account".into()),
    ///     AccountLogin::Email("you@example.com".into()),
    ///     AccountKind::Limits,
    ///     FoundIn::ProfileFolder(PathBuf::from("/home/user/.codex")),
    /// );
    ///
    /// let saved = SavedProfiles::default()
    ///     .with_account_name(&login, Some("ChatGPT · Home".into()));
    ///
    /// assert_eq!(saved.account_name(&login), Some("ChatGPT · Home"));
    /// ```
    pub fn with_account_name(
        self,
        account: &Account,
        name: Option<String>,
    ) -> Self {
        self.with_account_changed(account, |saved| saved.name = name)
    }

    /// The same content with `account` hidden or shown again. Only an
    /// account that can be saved ([`Account::can_be_saved`]) is hidden: for
    /// a key the content stays as it is.
    pub fn with_account_hidden(
        self,
        account: &Account,
        is_hidden: bool,
    ) -> Self {
        self.with_account_changed(account, |saved| saved.is_hidden = is_hidden)
    }

    /// The same content with an account the user added at `provider`,
    /// whose key is kept at `key` (spec 2026-11-19, Decision 7). Only the
    /// place of the key is saved. An account already added with the same
    /// place stays as it is.
    ///
    /// ```
    /// use otty_agents::{Account, AdminKeyRef, ProviderRef, SavedProfiles};
    ///
    /// let key = AdminKeyRef::EnvVar("DEEPSEEK_API_KEY".into());
    ///
    /// let saved = SavedProfiles::default()
    ///     .with_account_added(ProviderRef::of_id("deepseek"), key.clone());
    ///
    /// assert_eq!(
    ///     saved.added_accounts(),
    ///     [Account::added(ProviderRef::of_id("deepseek"), key)]
    /// );
    /// ```
    pub fn with_account_added(
        mut self,
        provider: ProviderRef,
        key: AdminKeyRef,
    ) -> Self {
        let account = Account::added(provider, key);
        if self.saved_account(&account).is_some() {
            return self;
        }

        if let Some(saved) = SavedAccount::new(&account) {
            self.accounts.push(saved);
        }
        self
    }

    /// The same content with the key of the account the user added,
    /// `account`, kept at `key` from now on; its name and whether it is
    /// hidden stay. An account OTTY found changes nothing.
    pub fn with_added_key(
        mut self,
        account: &Account,
        key: AdminKeyRef,
    ) -> Self {
        if account.added_key().is_none_or(|old| *old == key) {
            return self;
        }

        let moved = Account::added(account.provider().clone(), key);
        self.accounts.retain(|saved| !saved.is_of(&moved));
        if let Some(saved) =
            self.accounts.iter_mut().find(|saved| saved.is_of(account))
        {
            saved.identity = moved.identity().clone();
        }
        self
    }

    /// The same content without the account the user added, `account`,
    /// and its choices: Remove forgets it. An account OTTY found changes
    /// nothing.
    pub fn without_added_account(mut self, account: &Account) -> Self {
        if account.added_key().is_none() {
            return self;
        }

        self.accounts.retain(|saved| !saved.is_of(account));
        self
    }

    /// The same content with `profile` hidden or shown again, kept by its
    /// agent and its config folder.
    pub fn with_profile_hidden(
        mut self,
        profile: &Profile,
        is_hidden: bool,
    ) -> Self {
        self.hidden_profiles.retain(|hidden| !hidden.is_of(profile));
        if is_hidden {
            self.hidden_profiles.push(HiddenProfile::of(profile));
        }

        self
    }

    /// The same content with the admin key an older version kept in a
    /// profile moved to the organization of that profile, among the
    /// accounts `found` in each profile.
    ///
    /// An organization that already has an admin key keeps it, and the
    /// old one goes. The key of a profile with no organization found, such
    /// as one paid by a key alone or one logged out, stays in its entry
    /// and is moved by a later read.
    pub fn with_admin_keys_moved(
        mut self,
        found: &[(&Profile, &[Account])],
    ) -> Self {
        let old_keys = std::mem::take(&mut self.old_admin_keys);

        for old in old_keys {
            let organization = found
                .iter()
                .find(|(profile, _)| old.is_of(profile))
                .and_then(|(_, accounts)| {
                    accounts
                        .iter()
                        .find(|account| account.organization_id().is_some())
                });

            match organization {
                Some(organization) => {
                    if self.admin_key_of(organization).is_none() {
                        self = self.with_admin_key_of(
                            organization,
                            Some(old.admin_key),
                        );
                    }
                },
                None => self.old_admin_keys.push(old),
            }
        }

        self
    }

    /// The same content with each account kept by the host of its server
    /// kept by the id of the provider of `registry` that serves the host,
    /// so it is the choice of the account found for that id (spec
    /// 2026-11-19-agent-plugins, "Identity and links do not depend on a
    /// provider plugin"). The text written then names the id.
    pub fn with_providers_resolved(mut self, registry: &Registry) -> Self {
        for saved in &mut self.accounts {
            saved.provider = registry.resolve(&saved.provider);
        }

        self
    }

    /// The choices kept for `account`, if any.
    fn saved_account(&self, account: &Account) -> Option<&SavedAccount> {
        self.accounts.iter().find(|saved| saved.is_of(account))
    }

    /// The same content with the choices for `account` changed by
    /// `change`, in their place or in a new entry at the end. An entry
    /// left with nothing to keep goes, and an account that can not be
    /// saved changes nothing.
    fn with_account_changed(
        mut self,
        account: &Account,
        change: impl FnOnce(&mut SavedAccount),
    ) -> Self {
        let position =
            self.accounts.iter().position(|saved| saved.is_of(account));
        let mut saved = match position {
            Some(position) => self.accounts.remove(position),
            None => match SavedAccount::new(account) {
                Some(saved) => saved,
                None => return self,
            },
        };

        change(&mut saved);

        if !saved.is_empty() {
            let at = position.unwrap_or(self.accounts.len());
            self.accounts.insert(at, saved);
        }
        self
    }
}

/// What the user changed in the default profile of one agent. A part that
/// is `None` keeps the default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefaultProfileChanges {
    agent: AgentId,
    name: Option<String>,
    config_dir: Option<PathBuf>,
}

impl DefaultProfileChanges {
    /// The agent whose default profile is changed.
    pub fn agent(&self) -> &AgentId {
        &self.agent
    }

    /// The name the user gave the profile.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// The config folder the user moved the profile to.
    pub fn config_dir(&self) -> Option<&Path> {
        self.config_dir.as_deref()
    }

    /// No changes yet to the default profile of `agent`.
    ///
    /// ```
    /// use otty_agents::{AgentId, DefaultProfileChanges};
    ///
    /// let changes = DefaultProfileChanges::new(AgentId::new("codex"))
    ///     .with_name("Codex · Home");
    ///
    /// assert_eq!(changes.name(), Some("Codex · Home"));
    /// assert_eq!(changes.config_dir(), None);
    /// ```
    pub fn new(agent: AgentId) -> Self {
        Self {
            agent,
            name: None,
            config_dir: None,
        }
    }

    /// The same changes with a new name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// The same changes with a new config folder.
    pub fn with_config_dir(mut self, config_dir: PathBuf) -> Self {
        self.config_dir = Some(config_dir);
        self
    }
}

/// Why the content of `agents.json` could not be turned to or from text.
#[derive(Debug, thiserror::Error)]
pub enum SavedProfilesError {
    /// The text is not the layout of `agents.json`.
    #[error("the saved profiles can not be read: {0}")]
    Read(#[source] serde_json::Error),
    /// The content can not be written as text, because a path is not
    /// valid UTF-8 text.
    #[error("the profiles can not be saved: {0}")]
    Write(#[source] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::organization;

    fn claude_id() -> AgentId {
        AgentId::new("claude-code")
    }

    fn codex_id() -> AgentId {
        AgentId::new("codex")
    }

    /// A made-up admin key, never a real one.
    const MADE_UP_KEY: &str = "sk-ant-admin01-made-up-for-tests";

    fn content() -> SavedProfiles {
        SavedProfiles::default()
            .with_default_changes(
                DefaultProfileChanges::new(claude_id())
                    .with_name("Claude Code · Home")
                    .with_config_dir(PathBuf::from("/data/claude")),
            )
            .with_default_changes(
                DefaultProfileChanges::new(codex_id())
                    .with_name("Codex · Home"),
            )
            .with_added(Profile::added(
                claude_id(),
                "Claude Code · Work",
                PathBuf::from("/home/user/.claude-work"),
            ))
            .with_added(Profile::added(
                codex_id(),
                "Codex · Work",
                PathBuf::from("/home/user/.codex-work"),
            ))
            .with_admin_key_of(
                &organization("00000000-0000-4000-8000-00000000000b"),
                Some(AdminKeyRef::File(PathBuf::from(
                    "/home/user/.config/anthropic-admin-key",
                ))),
            )
    }

    fn load(text: &str) -> SavedProfiles {
        SavedProfiles::from_json(text)
            .expect("the text is the layout of agents.json")
    }

    /// The default Claude Code profile in `/home/user/.claude`.
    fn default_claude() -> Profile {
        Profile::default_for(
            claude_id(),
            "Claude Code",
            PathBuf::from("/home/user/.claude"),
        )
    }

    #[test]
    fn an_edited_default_profile_keeps_only_what_differs_from_the_agent() {
        let renamed_and_moved = Profile::default_for(
            claude_id(),
            "Claude Code · Home",
            PathBuf::from("/data/claude"),
        );

        let kept = SavedProfiles::default().with_default_edited(
            "Claude Code",
            &default_claude(),
            default_claude(),
        );
        let changed = SavedProfiles::default().with_default_edited(
            "Claude Code",
            &default_claude(),
            renamed_and_moved,
        );

        assert_eq!(
            kept,
            SavedProfiles::default()
                .with_default_changes(DefaultProfileChanges::new(claude_id()))
        );
        assert_eq!(
            changed,
            SavedProfiles::default().with_default_changes(
                DefaultProfileChanges::new(claude_id())
                    .with_name("Claude Code · Home")
                    .with_config_dir(PathBuf::from("/data/claude"))
            )
        );
    }

    /// The folder is written only when the user changed it, so a profile
    /// that follows the agent's variable keeps following it; a folder the
    /// user moved it to before stays.
    #[test]
    fn an_edit_that_keeps_the_folder_keeps_the_folder_moved_before() {
        let moved = Profile::default_for(
            claude_id(),
            "Claude Code",
            PathBuf::from("/data/claude"),
        );
        let saved = SavedProfiles::default().with_default_changes(
            DefaultProfileChanges::new(claude_id())
                .with_config_dir(PathBuf::from("/data/claude")),
        );
        let renamed = Profile::default_for(
            claude_id(),
            "Claude Code · Home",
            PathBuf::from("/data/claude"),
        );

        let edited = saved.with_default_edited("Claude Code", &moved, renamed);

        assert_eq!(
            edited,
            SavedProfiles::default().with_default_changes(
                DefaultProfileChanges::new(claude_id())
                    .with_name("Claude Code · Home")
                    .with_config_dir(PathBuf::from("/data/claude"))
            )
        );
    }

    #[test]
    fn saved_profiles_load_back_the_same() {
        let saved = content();

        let text = saved.to_json().expect("the paths are text");

        assert_eq!(load(&text), saved);
    }

    #[test]
    fn the_text_has_the_layout_of_the_module_doc() {
        let text = r#"{
            "default_profiles": {
                "claude-code": {
                    "name": "Claude Code · Home",
                    "config_dir": "/data/claude"
                },
                "codex": { "name": "Codex · Home" }
            },
            "added_profiles": [
                {
                    "agent": "claude-code",
                    "name": "Claude Code · Work",
                    "config_dir": "/home/user/.claude-work"
                },
                {
                    "agent": "codex",
                    "name": "Codex · Work",
                    "config_dir": "/home/user/.codex-work"
                }
            ],
            "accounts": [
                {
                    "provider": "anthropic",
                    "organization_id": "00000000-0000-4000-8000-00000000000b",
                    "admin_key": { "file": "/home/user/.config/anthropic-admin-key" }
                }
            ]
        }"#;

        assert_eq!(load(text), content());
    }

    #[test]
    fn an_old_file_with_hidden_and_a_program_loads_the_profiles_shown() {
        let text = r#"{
            "default_profiles": {
                "codex": { "name": "Codex · Home", "program": "/opt/codex/codex", "hidden": true }
            },
            "added_profiles": [
                {
                    "agent": "claude-code",
                    "name": "Claude Code · Work",
                    "config_dir": "/home/user/.claude-work",
                    "program": "/opt/claude/bin/claude",
                    "hidden": true
                }
            ]
        }"#;

        let saved = load(text);

        let expected = SavedProfiles::default()
            .with_default_changes(
                DefaultProfileChanges::new(codex_id())
                    .with_name("Codex · Home"),
            )
            .with_added(Profile::added(
                claude_id(),
                "Claude Code · Work",
                PathBuf::from("/home/user/.claude-work"),
            ));
        assert_eq!(saved, expected);
    }

    #[test]
    fn saving_writes_no_hidden_flag_and_no_program() {
        let text = content().to_json().expect("the paths are text");

        assert!(!text.contains("hidden"), "{text}");
        assert!(!text.contains("program"), "{text}");
    }

    /// Spec Decision 5: OTTY only shows, so the budget an older version
    /// saved is read without error and dropped on the next save.
    #[test]
    fn a_budget_in_an_old_file_is_read_and_dropped_on_save() {
        let text = r#"{
            "default_profiles": {
                "claude-code": { "name": "Claude Code · Home", "monthly_budget": 300.0 }
            },
            "added_profiles": [
                {
                    "agent": "codex",
                    "name": "Codex · Work",
                    "config_dir": "/home/user/.codex-work",
                    "monthly_budget": 49.5
                }
            ]
        }"#;

        let saved = load(text);
        let written = saved.to_json().expect("the paths are text");

        let expected = SavedProfiles::default()
            .with_default_changes(
                DefaultProfileChanges::new(claude_id())
                    .with_name("Claude Code · Home"),
            )
            .with_added(Profile::added(
                codex_id(),
                "Codex · Work",
                PathBuf::from("/home/user/.codex-work"),
            ));
        assert_eq!(saved, expected);
        assert!(!written.contains("budget"), "{written}");
    }

    #[test]
    fn only_the_place_of_the_admin_key_is_written_never_the_key() {
        let dir = crate::testing::TempDir::new();
        let key_file = dir.path().join("admin-key");
        std::fs::write(&key_file, MADE_UP_KEY).expect("the key file");
        let saved = SavedProfiles::default().with_admin_key_of(
            &organization("made-up-org"),
            Some(AdminKeyRef::File(key_file.clone())),
        );

        let text = saved.to_json().expect("the paths are text");

        assert!(text.contains(&format!("{}", key_file.display())));
        assert!(!text.contains(MADE_UP_KEY));
        assert!(!text.contains("sk-ant"));
    }

    /// Spec Decision 2: the Found by OTTY report opens on the first start
    /// only, so the file remembers it was shown.
    #[test]
    fn the_file_remembers_that_the_found_report_was_shown() {
        let shown = SavedProfiles::default().with_found_report_shown();

        let text = shown.to_json().expect("the paths are text");
        let not_shown = SavedProfiles::default()
            .to_json()
            .expect("the paths are text");

        assert!(text.contains(r#""found_report_shown": true"#), "{text}");
        assert!(load(&text).is_found_report_shown());
        assert!(!not_shown.contains("found_report"), "{not_shown}");
        assert!(!load(&not_shown).is_found_report_shown());
    }

    /// Spec 2026-11-19, Decision 8: the user allows the read of the
    /// system keyring once, and the file remembers it.
    #[test]
    fn the_file_remembers_that_the_keyring_may_be_read() {
        let allowed = SavedProfiles::default().with_keyring_allowed();

        let text = allowed.to_json().expect("the paths are text");
        let not_allowed = SavedProfiles::default()
            .to_json()
            .expect("the paths are text");

        assert!(text.contains(r#""keyring_allowed": true"#), "{text}");
        assert!(load(&text).is_keyring_allowed());
        assert!(!not_allowed.contains("keyring"), "{not_allowed}");
        assert!(!load(&not_allowed).is_keyring_allowed());
    }

    #[test]
    fn an_empty_text_is_no_changes() {
        assert_eq!(load(""), SavedProfiles::default());
        assert_eq!(load("  \n"), SavedProfiles::default());
        assert_eq!(load("{}"), SavedProfiles::default());
    }

    /// Spec 2026-11-19-agent-plugins, "`agents.json`": the entries of an
    /// agent or a provider no entry of the registry names, such as one a
    /// plugin served before or `moonshot`, are kept and written back, so
    /// they come back with their entry; an unknown field is skipped.
    #[test]
    fn the_entries_of_an_unknown_agent_or_provider_are_kept() {
        let text = r#"{
            "default_profiles": {
                "future-agent": { "name": "Future" },
                "codex": { "name": "Codex · Home", "colour": "blue" }
            },
            "added_profiles": [
                {
                    "agent": "future-agent",
                    "name": "Future · Work",
                    "config_dir": "/home/user/.future-work"
                },
                {
                    "agent": "claude-code",
                    "name": "Claude Code · Work",
                    "config_dir": "/home/user/.claude-work",
                    "colour": "blue"
                }
            ],
            "accounts": [
                {
                    "provider": "moonshot",
                    "key": { "env_var": "MOONSHOT_API_KEY" },
                    "name": "Kimi"
                },
                { "provider": "future", "account_id": "made-up", "hidden": true }
            ],
            "hidden_profiles": [
                { "agent": "goose", "config_dir": "/home/user/.config/goose" }
            ],
            "theme": "dark"
        }"#;
        let future = AgentId::new("future-agent");
        let goose = Profile::found(
            AgentId::new("goose"),
            "Goose",
            PathBuf::from("/home/user/.config/goose"),
        );
        let kimi = Account::added(
            ProviderRef::of_id("moonshot"),
            AdminKeyRef::EnvVar(String::from("MOONSHOT_API_KEY")),
        );
        let future_login = crate::testing::chatgpt_login("made-up");
        let future_login = Account::new(
            ProviderRef::of_id("future"),
            future_login.identity().clone(),
            future_login.login().clone(),
            future_login.kind(),
            future_login.found_in()[0].clone(),
        );

        let loaded = load(text);

        let changes = loaded.default_changes_of(&future);
        assert_eq!(
            changes.and_then(DefaultProfileChanges::name),
            Some("Future")
        );
        let added: Vec<_> = loaded
            .added()
            .iter()
            .map(|profile| profile.name())
            .collect();
        assert_eq!(added, ["Future · Work", "Claude Code · Work"]);
        assert_eq!(loaded.account_name(&kimi), Some("Kimi"));
        assert!(loaded.is_account_hidden(&future_login));
        assert!(loaded.is_profile_hidden(&goose));
        let written = loaded.to_json().expect("the paths are text");
        assert_eq!(load(&written), loaded);
        assert!(!written.contains("colour"), "{written}");
    }

    /// Spec 2026-11-19-agent-plugins, "`agents.json`": an account of a
    /// provider no entry serves may be kept by the host of its server;
    /// once an entry serves the host, it is the choice of the account found
    /// for its id.
    #[test]
    fn an_account_kept_by_host_is_the_account_of_the_provider_of_the_host() {
        let text = r#"{
            "accounts": [
                {
                    "provider": "host:api.deepseek.com",
                    "account_id": "made-up",
                    "name": "Gateway"
                }
            ]
        }"#;
        let login = |provider: ProviderRef| {
            let found = crate::testing::chatgpt_login("made-up");
            Account::new(
                provider,
                found.identity().clone(),
                found.login().clone(),
                found.kind(),
                found.found_in()[0].clone(),
            )
        };
        let by_host =
            login(ProviderRef::Host(String::from("api.deepseek.com")));
        let by_id = login(ProviderRef::of_id("deepseek"));
        let registry = Registry::built_in(&crate::testing::test_agents());

        let loaded = load(text);
        let resolved = loaded.clone().with_providers_resolved(&registry);

        assert_eq!(loaded.account_name(&by_host), Some("Gateway"));
        assert_eq!(loaded.account_name(&by_id), None);
        assert_eq!(resolved.account_name(&by_id), Some("Gateway"));
        let written = loaded.to_json().expect("the paths are text");
        assert!(written.contains(r#""provider": "host:api.deepseek.com""#));
    }

    #[test]
    fn a_text_that_is_not_this_layout_is_an_error() {
        let broken = SavedProfiles::from_json("{ not json");
        let wrong_type = SavedProfiles::from_json(r#"{"added_profiles":7}"#);

        assert!(matches!(broken, Err(SavedProfilesError::Read(_))));
        assert!(matches!(wrong_type, Err(SavedProfilesError::Read(_))));
    }

    #[cfg(unix)]
    #[test]
    fn a_path_that_is_not_text_can_not_be_saved() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        let path = PathBuf::from(OsStr::from_bytes(b"/home/user/.claude-\xff"));
        let saved = SavedProfiles::default().with_added(Profile::added(
            claude_id(),
            "Claude Code · Work",
            path,
        ));

        assert!(matches!(saved.to_json(), Err(SavedProfilesError::Write(_))));
    }

    #[test]
    fn an_added_profile_is_replaced_in_its_place() {
        let edited = Profile::added(
            claude_id(),
            "Claude Code · Office",
            PathBuf::from("/data/claude-office"),
        );

        let saved = content().with_added_replaced(
            &claude_id(),
            "Claude Code · Work",
            edited.clone(),
        );

        let names: Vec<_> = saved.added().iter().map(Profile::name).collect();
        assert_eq!(names, ["Claude Code · Office", "Codex · Work"]);
        assert_eq!(saved.added()[0], edited);
    }

    #[test]
    fn replacing_a_profile_that_is_not_saved_adds_it() {
        let profile = Profile::added(
            codex_id(),
            "Codex · New",
            PathBuf::from("/data/codex"),
        );

        let saved = SavedProfiles::default().with_added_replaced(
            &codex_id(),
            "Codex · Gone",
            profile.clone(),
        );

        assert_eq!(saved.added(), [profile]);
    }

    #[test]
    fn an_added_profile_is_removed_by_its_agent_and_name() {
        let saved = content()
            .without_added(&codex_id(), "Claude Code · Work")
            .without_added(&claude_id(), "Claude Code · Work");

        let names: Vec<_> = saved.added().iter().map(Profile::name).collect();
        assert_eq!(names, ["Codex · Work"]);
        assert_eq!(
            saved.default_changes_of(&codex_id()),
            content().default_changes_of(&codex_id())
        );
    }

    #[test]
    fn new_changes_of_an_agent_replace_its_earlier_ones() {
        let saved = SavedProfiles::default()
            .with_default_changes(
                DefaultProfileChanges::new(codex_id()).with_name("Codex · Old"),
            )
            .with_default_changes(
                DefaultProfileChanges::new(codex_id())
                    .with_config_dir(PathBuf::from("/data/codex")),
            );

        let changes = saved.default_changes_of(&codex_id());

        assert_eq!(
            changes,
            Some(
                &DefaultProfileChanges::new(codex_id())
                    .with_config_dir(PathBuf::from("/data/codex"))
            )
        );
        assert_eq!(saved.default_changes_of(&claude_id()), None);
    }
}

//! The content of `agents.json`: the profiles the user added, what they
//! changed in the default profiles, the user's names and hidden accounts,
//! the hidden profiles, and where the keys the user gave for the accounts
//! are kept (spec Decisions 1 and 7; spec 2026-11-19-agent-plugins, issue
//! 20).
//!
//! The file holds names and paths only, never a secret. Reading and writing
//! the file itself is the app's job; this module turns its content to and
//! from text. A default profile is listed under its agent's id with only
//! the parts the user changed; an added profile is a whole entry; an
//! account is listed by the plugin whose provider part found it and the
//! part's id of the account (spec 2026-11-23-accounts-apart-from-profiles,
//! "What OTTY stores"); a hidden profile by its agent and its config
//! folder:
//!
//! ```json
//! {
//!   "default_profiles": {
//!     "writer": {
//!       "name": "Writer · Home",
//!       "config_dir": "/data/writer"
//!     },
//!     "coder": { "name": "Coder · Home" }
//!   },
//!   "added_profiles": [
//!     {
//!       "agent": "writer",
//!       "name": "Writer · Semrush",
//!       "config_dir": "/home/user/.writer-semrush"
//!     }
//!   ],
//!   "accounts": [
//!     {
//!       "plugin": "example",
//!       "id": "org:00000000-0000-4000-8000-00000000000b",
//!       "name": "Acme",
//!       "given_keys": [
//!         { "slot": "admin", "file": "/home/user/.config/acme-key" }
//!       ]
//!     },
//!     { "plugin": "other", "id": "chatgpt:made-up-account", "hidden": true }
//!   ],
//!   "hidden_profiles": [
//!     { "agent": "coder", "config_dir": "/home/user/.coder-old" }
//!   ],
//!   "own_logins": [
//!     { "id": "6f0c1e2a", "plugin": "example", "provider": "example-cloud", "agent": "coder" }
//!   ],
//!   "found_report_shown": true,
//!   "keyring_allowed": true
//! }
//! ```
//!
//! An own login is OTTY's own login of a provider part (spec
//! 2026-11-23-accounts-apart-from-profiles, Decision 6), kept by its id;
//! its folder, `<data dir>/accounts/<id>/`, is not written. It is no
//! hidden profile: `hidden_profiles` are the profiles the user hid.
//!
//! `found_report_shown` tells that the Found by OTTY report was shown once,
//! so it does not open by itself again. `keyring_allowed` tells that the
//! user allowed the plugin parts that declare an entry of the system
//! keyring to read it. A hidden profile is kept by its folder, not by an
//! entry, so a default profile the user did not change can be hidden too.
//!
//! An account entry keeps only the user's choices: a name, Hide, and where
//! the keys the user gave for it are, each under the slot of the plugin
//! part that asked for it: a file or an environment variable; the key
//! itself is never written. The id is the part's own, which it keeps the
//! same between reads, so the choices survive a renewed token.
//!
//! Every part may be missing. The entries of an agent or a plugin no
//! entry of the registry names are kept as they are and not read, so they
//! come back when the plugin does (spec 2026-11-19-agent-plugins,
//! "`agents.json`"). An unknown field is skipped. Agents mode was never
//! released, so no earlier layout of the file is read (spec 2026-11-23,
//! Decision 12).

mod file_content;
mod hidden_profiles;
mod own_logins;
mod saved_accounts;

use std::path::{Path, PathBuf};

use self::file_content::FileContent;
use self::hidden_profiles::HiddenProfile;
use self::saved_accounts::SavedAccount;
use crate::account::Account;
use crate::agent::AgentId;
use crate::given_key::{GivenKey, KeyRef};
use crate::own_login::OwnLogin;
use crate::profile::Profile;

/// The content of `agents.json`: the profiles the user added, what they
/// changed in the default profiles, the user's names and hidden accounts,
/// the hidden profiles, where the keys the user gave for the accounts are
/// kept, and OTTY's own logins.
///
/// ```
/// use std::path::PathBuf;
///
/// use otty_agents::{AgentId, Profile, SavedProfiles};
///
/// let saved = SavedProfiles::default().with_added(Profile::added(
///     AgentId::new("coder"),
///     "Coder · Work",
///     PathBuf::from("/home/user/.coder-work"),
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
    hidden_profiles: Vec<HiddenProfile>,
    own_logins: Vec<OwnLogin>,
    is_found_report_shown: bool,
    is_keyring_allowed: bool,
}

impl SavedProfiles {
    /// OTTY's own logins, in the order they were made, of every plugin,
    /// whether or not it is installed.
    pub fn own_logins(&self) -> &[OwnLogin] {
        &self.own_logins
    }

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

    /// The keys the user gave for `account` after needs of plugin parts
    /// asked for them, each with its slot; empty when none.
    pub fn given_keys_of(&self, account: &Account) -> &[GivenKey] {
        self.saved_account(account)
            .map_or(&[], |saved| saved.given_keys.as_slice())
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

    /// Whether the user allowed the plugin parts that declare an entry of
    /// the system keyring to read it (spec 2026-11-19, Decision 8).
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
    /// let coder = Profile::default_for(
    ///     AgentId::new("coder"),
    ///     "Coder",
    ///     PathBuf::from("/home/user/.coder"),
    /// );
    /// let renamed = Profile::default_for(
    ///     AgentId::new("coder"),
    ///     "Coder · Home",
    ///     PathBuf::from("/home/user/.coder"),
    /// );
    ///
    /// let saved =
    ///     SavedProfiles::default().with_default_edited("Coder", &coder, renamed);
    /// let changes = saved.default_changes_of(&AgentId::new("coder"));
    ///
    /// assert_eq!(
    ///     changes.and_then(|changes| changes.name()),
    ///     Some("Coder · Home")
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

    /// The same content, remembering that the user allowed the plugin
    /// parts to read the system keyring.
    pub fn with_keyring_allowed(mut self) -> Self {
        self.is_keyring_allowed = true;
        self
    }

    /// The same content with one more own login, at the end.
    pub fn with_own_login(mut self, login: OwnLogin) -> Self {
        self.own_logins.push(login);
        self
    }

    /// The same content without the own login `id`. Only the entry goes:
    /// its folder is the app's to delete.
    pub fn without_own_login(mut self, id: &str) -> Self {
        self.own_logins.retain(|login| login.id() != id);
        self
    }

    /// The same content with one more profile the user added.
    pub fn with_added(mut self, profile: Profile) -> Self {
        self.added.push(profile);
        self
    }

    /// The same content with the added profile of `agent` named `name`
    /// replaced by `profile`, in its place. A profile that is not saved
    /// is added at the end.
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
        self
    }

    /// The same content with `key` as the place of the key the user gave
    /// for `account` under `slot`, in place of the one given before, or
    /// with no key under `slot`.
    ///
    /// ```
    /// use otty_agents::{
    ///     Account, AccountKind, GivenKey, KeyRef, ProviderId, SavedProfiles,
    /// };
    ///
    /// let organization = Account::new(
    ///     "example",
    ///     ProviderId::new("example-cloud"),
    ///     "org:made-up",
    ///     "Made-up org",
    ///     AccountKind::Organization,
    ///     Vec::new(),
    /// );
    /// let key = KeyRef::EnvVar("EXAMPLE_ORG_KEY".into());
    ///
    /// let saved = SavedProfiles::default().with_given_key_of(
    ///     &organization,
    ///     "admin",
    ///     Some(key.clone()),
    /// );
    ///
    /// assert_eq!(
    ///     saved.given_keys_of(&organization),
    ///     [GivenKey::new("admin", key)]
    /// );
    /// ```
    pub fn with_given_key_of(
        self,
        account: &Account,
        slot: &str,
        key: Option<KeyRef>,
    ) -> Self {
        self.with_account_changed(account, |saved| {
            saved.given_keys.retain(|given| given.slot() != slot);
            if let Some(key) = key {
                saved.given_keys.push(GivenKey::new(slot, key));
            }
        })
    }

    /// The same content with `name` as the user's name of `account`, or
    /// with none, so it is shown by the name its part gives again. The
    /// name is kept by the account's plugin and id, so it stays when the
    /// part reads the account again.
    ///
    /// ```
    /// use otty_agents::{Account, AccountKind, ProviderId, SavedProfiles};
    ///
    /// let login = Account::new(
    ///     "example",
    ///     ProviderId::new("other-cloud"),
    ///     "chatgpt:made-up-account",
    ///     "you@example.com",
    ///     AccountKind::Subscription,
    ///     Vec::new(),
    /// );
    ///
    /// let saved = SavedProfiles::default()
    ///     .with_account_name(&login, Some("Login · Home".into()));
    ///
    /// assert_eq!(saved.account_name(&login), Some("Login · Home"));
    /// ```
    pub fn with_account_name(
        self,
        account: &Account,
        name: Option<String>,
    ) -> Self {
        self.with_account_changed(account, |saved| saved.name = name)
    }

    /// The same content with `account` hidden or shown again, kept by its
    /// plugin and id.
    pub fn with_account_hidden(
        self,
        account: &Account,
        is_hidden: bool,
    ) -> Self {
        self.with_account_changed(account, |saved| saved.is_hidden = is_hidden)
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

    /// The choices kept for `account`, if any.
    fn saved_account(&self, account: &Account) -> Option<&SavedAccount> {
        self.accounts.iter().find(|saved| saved.is_of(account))
    }

    /// The same content with the choices for `account` changed by
    /// `change`, in their place or in a new entry at the end. An entry
    /// left with nothing to keep goes.
    fn with_account_changed(
        mut self,
        account: &Account,
        change: impl FnOnce(&mut SavedAccount),
    ) -> Self {
        let position =
            self.accounts.iter().position(|saved| saved.is_of(account));
        let mut saved = match position {
            Some(position) => self.accounts.remove(position),
            None => SavedAccount::new(account),
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
    /// let changes = DefaultProfileChanges::new(AgentId::new("coder"))
    ///     .with_name("Coder · Home");
    ///
    /// assert_eq!(changes.name(), Some("Coder · Home"));
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

    /// A made-up key, never a real one.
    const MADE_UP_KEY: &str = "sk-made-up-for-tests-0001";

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
            .with_given_key_of(
                &organization("00000000-0000-4000-8000-00000000000b"),
                "admin",
                Some(KeyRef::File(PathBuf::from(
                    "/home/user/.config/anthropic-org-key",
                ))),
            )
    }

    fn load(text: &str) -> SavedProfiles {
        SavedProfiles::from_json(text)
            .expect("the text is the layout of agents.json")
    }

    /// The default Writer profile in `/home/user/.writer`.
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
                    "plugin": "example",
                    "id": "org:00000000-0000-4000-8000-00000000000b",
                    "given_keys": [
                        { "slot": "admin", "file": "/home/user/.config/anthropic-org-key" }
                    ]
                }
            ]
        }"#;

        assert_eq!(load(text), content());
    }

    #[test]
    fn only_the_place_of_a_given_key_is_written_never_the_key() {
        let dir = crate::testing::TempDir::new();
        let key_file = dir.path().join("org-key");
        std::fs::write(&key_file, MADE_UP_KEY).expect("the key file");
        let saved = SavedProfiles::default().with_given_key_of(
            &organization("made-up-org"),
            "admin",
            Some(KeyRef::File(key_file.clone())),
        );

        let text = saved.to_json().expect("the paths are text");

        assert!(text.contains(&format!("{}", key_file.display())));
        assert!(!text.contains(MADE_UP_KEY));
        assert!(!text.contains("sk-made-up"));
    }

    /// Issue 20: a key given again under the same slot replaces the one
    /// before; other slots stay, and none removes the slot.
    #[test]
    fn a_key_given_again_under_its_slot_replaces_the_one_before() {
        let organization = organization("made-up-org");
        let place = |name: &str| KeyRef::EnvVar(name.to_string());

        let saved = SavedProfiles::default()
            .with_given_key_of(&organization, "admin", Some(place("FIRST")))
            .with_given_key_of(&organization, "other", Some(place("OTHER")))
            .with_given_key_of(&organization, "admin", Some(place("SECOND")));
        let removed =
            saved
                .clone()
                .with_given_key_of(&organization, "admin", None);

        assert_eq!(
            saved.given_keys_of(&organization),
            [
                GivenKey::new("other", place("OTHER")),
                GivenKey::new("admin", place("SECOND")),
            ]
        );
        assert_eq!(
            removed.given_keys_of(&organization),
            [GivenKey::new("other", place("OTHER"))]
        );
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
    /// agent or a plugin no entry of the registry names, such as one the
    /// user removed, are kept and written back, so they come back with
    /// the plugin; an unknown field is skipped.
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
                { "plugin": "future", "id": "chatgpt:made-up", "hidden": true }
            ],
            "hidden_profiles": [
                { "agent": "gone-agent", "config_dir": "/home/user/.config/gone-agent" }
            ],
            "theme": "dark"
        }"#;
        let future = AgentId::new("future-agent");
        let gone = Profile::added(
            AgentId::new("gone-agent"),
            "Gone Agent",
            PathBuf::from("/home/user/.config/gone-agent"),
        );
        let future_login = Account::new(
            "future",
            crate::provider::ProviderId::new("openai"),
            "chatgpt:made-up",
            "you@example.com",
            crate::account::AccountKind::Subscription,
            Vec::new(),
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
        assert!(loaded.is_account_hidden(&future_login));
        assert!(loaded.is_profile_hidden(&gone));
        let written = loaded.to_json().expect("the paths are text");
        assert_eq!(load(&written), loaded);
        assert!(!written.contains("colour"), "{written}");
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

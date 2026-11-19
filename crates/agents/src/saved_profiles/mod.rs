//! The content of `agents.json`: the profiles the user added, what they
//! changed in the default profiles, and where the admin keys of the
//! organizations are kept (spec Decisions 1 and 7).
//!
//! The file holds names and paths only, never a secret. Reading and writing
//! the file itself is the app's job; this module turns its content to and
//! from text. A default profile is listed under its agent's id with only
//! the parts the user changed; an added profile is a whole entry; an
//! organization is listed by its provider and its id:
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
//!       "admin_key": { "file": "/home/user/.config/anthropic-admin-key" }
//!     }
//!   ],
//!   "found_report_shown": true
//! }
//! ```
//!
//! `found_report_shown` tells that the Found by OTTY report was shown once,
//! so it does not open by itself again. A profile OTTY found is not
//! written; once the user changes it, it is saved as an added profile.
//!
//! An organization paid per token may keep where its admin key is, a file
//! or an environment variable; the key itself is never written. An account
//! is written by its organization id only, never by anything taken from a
//! key.
//!
//! Older versions kept the admin key in the entry of a profile. Such a key
//! is moved to the organization of the profile by
//! [`SavedProfiles::with_admin_keys_moved`] once the accounts of the
//! profiles are known, and stays in its entry until then.
//!
//! Every part may be missing. A file from a newer version loads without
//! the entries of an agent this version does not know, and an unknown field
//! is skipped. So is what older versions wrote and this one no longer
//! keeps: `hidden` (every profile is shown), `program` (the program is
//! found by its name) and `monthly_budget` (OTTY only shows, spec
//! Decision 5).

mod admin_keys;
mod file_content;

use std::path::{Path, PathBuf};

use self::admin_keys::{AccountAdminKey, OldAdminKey, OldKeyOwner};
use self::file_content::FileContent;
use crate::account::Account;
use crate::admin_key::AdminKeyRef;
use crate::agent::{Agent, AgentId};
use crate::profile::Profile;

/// The content of `agents.json`: the profiles the user added, what they
/// changed in the default profiles, and where the admin keys of the
/// organizations are kept.
///
/// ```
/// use std::path::PathBuf;
///
/// use otty_agents::{AgentId, Profile, SavedProfiles, known_agents};
///
/// let saved = SavedProfiles::default().with_added(Profile::added(
///     AgentId::new("codex"),
///     "Codex · Work",
///     PathBuf::from("/home/user/.codex-work"),
/// ));
///
/// let text = saved.to_json().expect("the paths are text");
/// let loaded = SavedProfiles::from_json(&text, &known_agents());
///
/// assert_eq!(loaded.expect("the text was just written"), saved);
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SavedProfiles {
    default_changes: Vec<DefaultProfileChanges>,
    added: Vec<Profile>,
    admin_keys: Vec<AccountAdminKey>,
    old_admin_keys: Vec<OldAdminKey>,
    is_found_report_shown: bool,
}

impl SavedProfiles {
    /// What the user changed in the default profile of `agent`, if
    /// anything.
    pub fn default_changes_of(
        &self,
        agent: AgentId,
    ) -> Option<&DefaultProfileChanges> {
        self.default_changes
            .iter()
            .find(|changes| changes.agent == agent)
    }

    /// The profiles the user added, in the order they were added.
    pub fn added(&self) -> &[Profile] {
        &self.added
    }

    /// Where the admin key of `account` is kept, when the user gave one;
    /// only an organization has one (spec Decision 7).
    pub fn admin_key_of(&self, account: &Account) -> Option<&AdminKeyRef> {
        self.admin_keys
            .iter()
            .find(|saved| saved.is_of(account))
            .map(|saved| &saved.admin_key)
    }

    /// Whether the Found by OTTY report was shown once: it opens by itself
    /// on the first start only (spec Decision 2).
    pub fn is_found_report_shown(&self) -> bool {
        self.is_found_report_shown
    }

    /// The content read from the text of `agents.json`.
    ///
    /// An empty text is no changes. The entries of an agent not in
    /// `agents` are left out, and unknown fields are skipped. Only text
    /// that is not this layout at all is an error.
    pub fn from_json(
        text: &str,
        agents: &[Box<dyn Agent>],
    ) -> Result<Self, SavedProfilesError> {
        if text.trim().is_empty() {
            return Ok(Self::default());
        }

        let file: FileContent =
            serde_json::from_str(text).map_err(SavedProfilesError::Read)?;

        Ok(file.into_saved(agents))
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
    /// let changes = saved.default_changes_of(AgentId::new("codex"));
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

        let mut changes = DefaultProfileChanges::new(agent);
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
        agent: AgentId,
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
    pub fn without_added(mut self, agent: AgentId, name: &str) -> Self {
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
    ///     FoundIn, Provider, SavedProfiles,
    /// };
    ///
    /// let organization = Account::new(
    ///     Provider::Anthropic,
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
        mut self,
        account: &Account,
        admin_key: Option<AdminKeyRef>,
    ) -> Self {
        let Some(organization_id) = account.organization_id() else {
            return self;
        };

        self.admin_keys.retain(|saved| !saved.is_of(account));
        if let Some(admin_key) = admin_key {
            self.admin_keys.push(AccountAdminKey {
                provider: account.provider(),
                organization_id: organization_id.to_string(),
                admin_key,
            });
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
    pub fn agent(&self) -> AgentId {
        self.agent
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
    use crate::known_agents;
    use crate::testing::organization;

    const CLAUDE: AgentId = AgentId::new("claude-code");
    const CODEX: AgentId = AgentId::new("codex");

    /// A made-up admin key, never a real one.
    const MADE_UP_KEY: &str = "sk-ant-admin01-made-up-for-tests";

    fn content() -> SavedProfiles {
        SavedProfiles::default()
            .with_default_changes(
                DefaultProfileChanges::new(CLAUDE)
                    .with_name("Claude Code · Home")
                    .with_config_dir(PathBuf::from("/data/claude")),
            )
            .with_default_changes(
                DefaultProfileChanges::new(CODEX).with_name("Codex · Home"),
            )
            .with_added(Profile::added(
                CLAUDE,
                "Claude Code · Work",
                PathBuf::from("/home/user/.claude-work"),
            ))
            .with_added(Profile::added(
                CODEX,
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
        SavedProfiles::from_json(text, &known_agents())
            .expect("the text is the layout of agents.json")
    }

    /// The default Claude Code profile in `/home/user/.claude`.
    fn default_claude() -> Profile {
        Profile::default_for(
            CLAUDE,
            "Claude Code",
            PathBuf::from("/home/user/.claude"),
        )
    }

    #[test]
    fn an_edited_default_profile_keeps_only_what_differs_from_the_agent() {
        let renamed_and_moved = Profile::default_for(
            CLAUDE,
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
                .with_default_changes(DefaultProfileChanges::new(CLAUDE))
        );
        assert_eq!(
            changed,
            SavedProfiles::default().with_default_changes(
                DefaultProfileChanges::new(CLAUDE)
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
            CLAUDE,
            "Claude Code",
            PathBuf::from("/data/claude"),
        );
        let saved = SavedProfiles::default().with_default_changes(
            DefaultProfileChanges::new(CLAUDE)
                .with_config_dir(PathBuf::from("/data/claude")),
        );
        let renamed = Profile::default_for(
            CLAUDE,
            "Claude Code · Home",
            PathBuf::from("/data/claude"),
        );

        let edited = saved.with_default_edited("Claude Code", &moved, renamed);

        assert_eq!(
            edited,
            SavedProfiles::default().with_default_changes(
                DefaultProfileChanges::new(CLAUDE)
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
                DefaultProfileChanges::new(CODEX).with_name("Codex · Home"),
            )
            .with_added(Profile::added(
                CLAUDE,
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
                DefaultProfileChanges::new(CLAUDE)
                    .with_name("Claude Code · Home"),
            )
            .with_added(Profile::added(
                CODEX,
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

    #[test]
    fn an_empty_text_is_no_changes() {
        assert_eq!(load(""), SavedProfiles::default());
        assert_eq!(load("  \n"), SavedProfiles::default());
        assert_eq!(load("{}"), SavedProfiles::default());
    }

    #[test]
    fn an_unknown_agent_or_field_is_left_out() {
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
            "theme": "dark"
        }"#;

        let expected = SavedProfiles::default()
            .with_default_changes(
                DefaultProfileChanges::new(CODEX).with_name("Codex · Home"),
            )
            .with_added(Profile::added(
                CLAUDE,
                "Claude Code · Work",
                PathBuf::from("/home/user/.claude-work"),
            ));
        assert_eq!(load(text), expected);
    }

    #[test]
    fn a_text_that_is_not_this_layout_is_an_error() {
        let agents = known_agents();

        let broken = SavedProfiles::from_json("{ not json", &agents);
        let wrong_type =
            SavedProfiles::from_json(r#"{"added_profiles":7}"#, &agents);

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
            CLAUDE,
            "Claude Code · Work",
            path,
        ));

        assert!(matches!(saved.to_json(), Err(SavedProfilesError::Write(_))));
    }

    #[test]
    fn an_added_profile_is_replaced_in_its_place() {
        let edited = Profile::added(
            CLAUDE,
            "Claude Code · Office",
            PathBuf::from("/data/claude-office"),
        );

        let saved = content().with_added_replaced(
            CLAUDE,
            "Claude Code · Work",
            edited.clone(),
        );

        let names: Vec<_> = saved.added().iter().map(Profile::name).collect();
        assert_eq!(names, ["Claude Code · Office", "Codex · Work"]);
        assert_eq!(saved.added()[0], edited);
    }

    #[test]
    fn replacing_a_profile_that_is_not_saved_adds_it() {
        let profile =
            Profile::added(CODEX, "Codex · New", PathBuf::from("/data/codex"));

        let saved = SavedProfiles::default().with_added_replaced(
            CODEX,
            "Codex · Gone",
            profile.clone(),
        );

        assert_eq!(saved.added(), [profile]);
    }

    #[test]
    fn an_added_profile_is_removed_by_its_agent_and_name() {
        let saved = content()
            .without_added(CODEX, "Claude Code · Work")
            .without_added(CLAUDE, "Claude Code · Work");

        let names: Vec<_> = saved.added().iter().map(Profile::name).collect();
        assert_eq!(names, ["Codex · Work"]);
        assert_eq!(
            saved.default_changes_of(CODEX),
            content().default_changes_of(CODEX)
        );
    }

    #[test]
    fn new_changes_of_an_agent_replace_its_earlier_ones() {
        let saved = SavedProfiles::default()
            .with_default_changes(
                DefaultProfileChanges::new(CODEX).with_name("Codex · Old"),
            )
            .with_default_changes(
                DefaultProfileChanges::new(CODEX)
                    .with_config_dir(PathBuf::from("/data/codex")),
            );

        let changes = saved.default_changes_of(CODEX);

        assert_eq!(
            changes,
            Some(
                &DefaultProfileChanges::new(CODEX)
                    .with_config_dir(PathBuf::from("/data/codex"))
            )
        );
        assert_eq!(saved.default_changes_of(CLAUDE), None);
    }
}

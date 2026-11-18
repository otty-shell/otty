//! The content of `agents.json`: the profiles the user added and what they
//! changed in the default profiles (spec Decision 1).
//!
//! The file holds names and paths only, never a secret. Reading and writing
//! the file itself is the app's job; this module turns its content to and
//! from text. A default profile is listed under its agent's id with only
//! the parts the user changed; an added profile is a whole entry:
//!
//! ```json
//! {
//!   "default_profiles": {
//!     "claude-code": {
//!       "name": "Claude Code · Home",
//!       "config_dir": "/data/claude",
//!       "admin_key": { "file": "/home/user/.config/anthropic-admin-key" },
//!       "monthly_budget": 300.0
//!     },
//!     "codex": { "name": "Codex · Home" }
//!   },
//!   "added_profiles": [
//!     {
//!       "agent": "claude-code",
//!       "name": "Claude Code · Semrush",
//!       "config_dir": "/home/user/.claude-semrush",
//!       "admin_key": { "env_var": "ANTHROPIC_ADMIN_KEY" },
//!       "monthly_budget": 49.5
//!     }
//!   ]
//! }
//! ```
//!
//! An API key profile may keep where its admin key is, a file or an
//! environment variable, and a monthly budget in dollars (spec Decision 9);
//! the key itself is never written.
//!
//! Every part may be missing. A file from a newer version loads without
//! the entries of an agent this version does not know, and an unknown field
//! is skipped. So is what older versions wrote and this one no longer
//! keeps: `hidden` (every profile is shown) and `program` (the program is
//! found by its name).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::admin_key::AdminKeyRef;
use crate::agent::{Agent, AgentId};
use crate::dollars::Dollars;
use crate::profile::Profile;

/// The content of `agents.json`: the profiles the user added and what they
/// changed in the default profiles.
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

        let mut saved = Self::default();
        for (agent, entry) in file.default_profiles {
            if let Some(agent) = known_agent_id(&agent, agents) {
                saved = saved.with_default_changes(entry.into_changes(agent));
            }
        }
        for entry in file.added_profiles {
            if let Some(agent) = known_agent_id(&entry.agent, agents) {
                saved = saved.with_added(entry.into_profile(agent));
            }
        }

        Ok(saved)
    }

    /// The text of `agents.json`.
    ///
    /// It fails only when a path is not valid UTF-8 text.
    pub fn to_json(&self) -> Result<String, SavedProfilesError> {
        let file = FileContent {
            default_profiles: self
                .default_changes
                .iter()
                .map(|changes| {
                    let agent = String::from(changes.agent.as_str());
                    (agent, DefaultEntry::from_changes(changes))
                })
                .collect(),
            added_profiles: self
                .added
                .iter()
                .map(AddedEntry::from_profile)
                .collect(),
        };

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
        agent: AgentId,
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
    pub fn without_added(mut self, agent: AgentId, name: &str) -> Self {
        self.added
            .retain(|saved| !(saved.agent() == agent && saved.name() == name));
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
    admin_key: Option<AdminKeyRef>,
    monthly_budget: Option<Dollars>,
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

    /// Where the admin key of the profile is kept.
    pub fn admin_key(&self) -> Option<&AdminKeyRef> {
        self.admin_key.as_ref()
    }

    /// The monthly budget the user set.
    pub fn monthly_budget(&self) -> Option<Dollars> {
        self.monthly_budget
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
            admin_key: None,
            monthly_budget: None,
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

    /// The same changes with the place of the admin key, or none.
    pub fn with_admin_key(mut self, admin_key: Option<AdminKeyRef>) -> Self {
        self.admin_key = admin_key;
        self
    }

    /// The same changes with a monthly budget, or none.
    pub fn with_monthly_budget(mut self, budget: Option<Dollars>) -> Self {
        self.monthly_budget = budget;
        self
    }
}

/// The text of `agents.json` as it is read and written.
#[derive(Debug, Default, Serialize, Deserialize)]
struct FileContent {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    default_profiles: BTreeMap<String, DefaultEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    added_profiles: Vec<AddedEntry>,
}

/// The changes to one default profile in the text.
#[derive(Debug, Serialize, Deserialize)]
struct DefaultEntry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    config_dir: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    admin_key: Option<AdminKeyEntry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    monthly_budget: Option<f64>,
}

impl DefaultEntry {
    fn from_changes(changes: &DefaultProfileChanges) -> Self {
        Self {
            name: changes.name.clone(),
            config_dir: changes.config_dir.clone(),
            admin_key: changes.admin_key.as_ref().map(AdminKeyEntry::from_ref),
            monthly_budget: changes
                .monthly_budget
                .map(|budget| budget.dollars()),
        }
    }

    fn into_changes(self, agent: AgentId) -> DefaultProfileChanges {
        DefaultProfileChanges {
            agent,
            name: self.name,
            config_dir: self.config_dir,
            admin_key: self.admin_key.and_then(AdminKeyEntry::into_ref),
            monthly_budget: self.monthly_budget.and_then(Dollars::from_dollars),
        }
    }
}

/// One added profile in the text.
#[derive(Debug, Serialize, Deserialize)]
struct AddedEntry {
    agent: String,
    name: String,
    config_dir: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    admin_key: Option<AdminKeyEntry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    monthly_budget: Option<f64>,
}

impl AddedEntry {
    fn from_profile(profile: &Profile) -> Self {
        Self {
            agent: String::from(profile.agent().as_str()),
            name: String::from(profile.name()),
            config_dir: profile.config_dir().to_path_buf(),
            admin_key: profile.admin_key().map(AdminKeyEntry::from_ref),
            monthly_budget: profile
                .monthly_budget()
                .map(|budget| budget.dollars()),
        }
    }

    fn into_profile(self, agent: AgentId) -> Profile {
        Profile::added(agent, self.name, self.config_dir)
            .with_admin_key(self.admin_key.and_then(AdminKeyEntry::into_ref))
            .with_monthly_budget(
                self.monthly_budget.and_then(Dollars::from_dollars),
            )
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

/// The id of the agent of `agents` saved as `id`, if it is one of them.
fn known_agent_id(id: &str, agents: &[Box<dyn Agent>]) -> Option<AgentId> {
    agents
        .iter()
        .map(|agent| agent.descriptor().id())
        .find(|known| known.as_str() == id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::known_agents;

    const CLAUDE: AgentId = AgentId::new("claude-code");
    const CODEX: AgentId = AgentId::new("codex");

    /// A made-up admin key, never a real one.
    const MADE_UP_KEY: &str = "sk-ant-admin01-made-up-for-tests";

    fn budget(dollars: u64) -> Option<Dollars> {
        Some(Dollars::from_micro_dollars(dollars * 1_000_000))
    }

    fn content() -> SavedProfiles {
        SavedProfiles::default()
            .with_default_changes(
                DefaultProfileChanges::new(CLAUDE)
                    .with_name("Claude Code · Home")
                    .with_config_dir(PathBuf::from("/data/claude"))
                    .with_admin_key(Some(AdminKeyRef::File(PathBuf::from(
                        "/home/user/.config/anthropic-admin-key",
                    ))))
                    .with_monthly_budget(budget(300)),
            )
            .with_default_changes(
                DefaultProfileChanges::new(CODEX).with_name("Codex · Home"),
            )
            .with_added(
                Profile::added(
                    CLAUDE,
                    "Claude Code · Work",
                    PathBuf::from("/home/user/.claude-work"),
                )
                .with_admin_key(Some(AdminKeyRef::EnvVar(String::from(
                    "ANTHROPIC_ADMIN_KEY",
                ))))
                .with_monthly_budget(Some(
                    Dollars::from_micro_dollars(49_500_000),
                )),
            )
            .with_added(
                Profile::added(
                    CODEX,
                    "Codex · Work",
                    PathBuf::from("/home/user/.codex-work"),
                )
                .with_monthly_budget(budget(20)),
            )
    }

    fn load(text: &str) -> SavedProfiles {
        SavedProfiles::from_json(text, &known_agents())
            .expect("the text is the layout of agents.json")
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
                    "config_dir": "/data/claude",
                    "admin_key": { "file": "/home/user/.config/anthropic-admin-key" },
                    "monthly_budget": 300
                },
                "codex": { "name": "Codex · Home" }
            },
            "added_profiles": [
                {
                    "agent": "claude-code",
                    "name": "Claude Code · Work",
                    "config_dir": "/home/user/.claude-work",
                    "admin_key": { "env_var": "ANTHROPIC_ADMIN_KEY" },
                    "monthly_budget": 49.5
                },
                {
                    "agent": "codex",
                    "name": "Codex · Work",
                    "config_dir": "/home/user/.codex-work",
                    "monthly_budget": 20.0
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

    #[test]
    fn a_file_without_an_admin_key_or_a_budget_loads_without_them() {
        let text = r#"{
            "default_profiles": { "claude-code": { "name": "Claude Code · Home" } },
            "added_profiles": [
                {
                    "agent": "claude-code",
                    "name": "Claude Code · Work",
                    "config_dir": "/home/user/.claude-work"
                }
            ]
        }"#;

        let saved = load(text);

        let changes = saved.default_changes_of(CLAUDE).expect("the changes");
        assert_eq!(changes.admin_key(), None);
        assert_eq!(changes.monthly_budget(), None);
        assert_eq!(saved.added()[0].admin_key(), None);
        assert_eq!(saved.added()[0].monthly_budget(), None);
    }

    #[test]
    fn an_admin_key_of_no_kind_or_a_budget_below_zero_is_left_out() {
        let text = r#"{
            "added_profiles": [
                {
                    "agent": "claude-code",
                    "name": "Claude Code · Work",
                    "config_dir": "/home/user/.claude-work",
                    "admin_key": { "keychain": "anthropic" },
                    "monthly_budget": -5
                }
            ]
        }"#;

        let saved = load(text);

        assert_eq!(saved.added()[0].admin_key(), None);
        assert_eq!(saved.added()[0].monthly_budget(), None);
    }

    #[test]
    fn only_the_place_of_the_admin_key_is_written_never_the_key() {
        let dir = crate::testing::TempDir::new();
        let key_file = dir.path().join("admin-key");
        std::fs::write(&key_file, MADE_UP_KEY).expect("the key file");
        let saved = SavedProfiles::default().with_added(
            Profile::added(
                CLAUDE,
                "Claude Code · API",
                PathBuf::from("/home/user/.claude-api"),
            )
            .with_admin_key(Some(AdminKeyRef::File(key_file.clone()))),
        );

        let text = saved.to_json().expect("the paths are text");

        assert!(text.contains(&format!("{}", key_file.display())));
        assert!(!text.contains(MADE_UP_KEY));
        assert!(!text.contains("sk-ant"));
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
        )
        .with_monthly_budget(budget(10));

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

//! OpenCode and Kilo as [`Agent`]s: one type for both, told apart by
//! their [`Fork`].

use std::path::PathBuf;
use std::sync::Arc;

use super::accounts;
use super::database::Database;
use super::fork::{Fork, KILO, OPENCODE};
use super::location::DatabaseLocation;
use super::sessions::{DatabaseSessions, newest_version};
use super::spend::DatabaseSpend;
use super::stats::DatabaseStats;
use crate::account::{Account, AccountKind};
use crate::account_key::AccountKey;
use crate::agent::{Agent, AgentEntry, ConfigFolder};
use crate::bill::BillSource;
use crate::command::AgentCommand;
use crate::env::AgentEnv;
use crate::hidden_terminal::StartTerminal;
use crate::limits::{BillingKind, LimitSource};
use crate::profile::{Profile, ProfileStatus};
use crate::program::StartProgram;
use crate::session::{SessionId, SessionSource};
use crate::source_id::SourceId;
use crate::spend::SpendCounter;
use crate::stats::StatsSource;

/// The ids of the session sources, in the order they are given.
const SESSION_SOURCES: [SourceId; 1] = [SourceId::AgentDatabase];

/// OpenCode, or Kilo, a fork of it with the same files.
///
/// Both keep their logins and sessions in one data folder, such as
/// `~/.local/share/kilo`, which is the folder of a profile. Their database
/// is read again on each refresh; it is small next to the session files of
/// other agents, and OTTY never writes to it.
///
/// ```
/// use otty_agents::{Agent, OpenCodeAgent};
///
/// assert_eq!(OpenCodeAgent::kilo().entry().name(), "Kilo");
/// assert_eq!(OpenCodeAgent::opencode().entry().program(), "opencode");
/// ```
#[derive(Debug)]
pub struct OpenCodeAgent {
    entry: AgentEntry,
    location: Arc<DatabaseLocation>,
}

impl OpenCodeAgent {
    /// The fork this agent is.
    fn fork(&self) -> &'static Fork {
        self.location.fork()
    }

    /// Kilo, by Kilo Code.
    pub fn kilo() -> Self {
        Self::of(&KILO)
    }

    /// OpenCode, by Anomaly.
    pub fn opencode() -> Self {
        Self::of(&OPENCODE)
    }

    fn of(fork: &'static Fork) -> Self {
        Self {
            entry: fork.entry(),
            location: Arc::new(DatabaseLocation::new(fork)),
        }
    }

    /// The database of `profile`, when it is there and can be read.
    fn database(&self, profile: &Profile) -> Option<Database> {
        let path = self.location.database_of(profile);

        Database::open(&path).ok().flatten()
    }
}

impl Agent for OpenCodeAgent {
    fn entry(&self) -> &AgentEntry {
        &self.entry
    }

    fn config_folder(&self) -> ConfigFolder {
        self.fork().config_folder()
    }

    /// Yes: the database keeps what each answer cost.
    fn writes_cost(&self) -> bool {
        true
    }

    /// The agent's folder in `XDG_DATA_HOME`, or in `~/.local/share`.
    fn default_config_dir(&self, env: &AgentEnv) -> PathBuf {
        self.location.default_data_dir(env)
    }

    /// Connected with any login it keeps; paid per token when one of them
    /// is a key. Looking for the program on `PATH` is left to the app.
    fn detect(&self, profile: &Profile) -> ProfileStatus {
        if !profile.config_dir().is_dir() {
            return ProfileStatus::ConfigDirNotFound;
        }

        let found = self.accounts(profile);
        if found.is_empty() {
            return ProfileStatus::NotLoggedIn;
        }

        let pays_per_token = found
            .iter()
            .any(|account| account.kind() == AccountKind::Money);

        ProfileStatus::Connected {
            account: None,
            plan: None,
            billing: pays_per_token.then_some(BillingKind::ApiKey),
        }
    }

    fn accounts(&self, profile: &Profile) -> Vec<Account> {
        let database = self.database(profile);

        accounts::accounts(profile.config_dir(), database.as_ref())
    }

    /// The key or the login token of the account in the logins of the
    /// data folder and the database: a key, the token of a Kilo login with
    /// its team, or the access token of a ChatGPT login.
    fn account_key(
        &self,
        profile: &Profile,
        account: &Account,
    ) -> Option<AccountKey> {
        let database = self.database(profile);

        accounts::account_key(profile.config_dir(), database.as_ref(), account)
    }

    /// Whether its database lists a session.
    fn has_sessions(&self, profile: &Profile) -> bool {
        self.database(profile)
            .and_then(|database| database.sessions().ok())
            .is_some_and(|sessions| !sessions.is_empty())
    }

    /// The version of the agent that wrote the session that changed last.
    fn version(&self, profile: &Profile) -> Option<String> {
        let sessions = self.database(profile)?.sessions().ok()?;

        newest_version(&sessions)
    }

    /// `kilo` for a new session, `kilo -s <id>` to continue one.
    fn launch(
        &self,
        profile: &Profile,
        session: Option<&SessionId>,
    ) -> AgentCommand {
        let command = self.fork().start_command(profile);

        match session {
            Some(id) => command.with_arg("-s").with_arg(id.as_str()),
            None => command,
        }
    }

    /// `kilo auth login`.
    fn login(&self, profile: &Profile) -> AgentCommand {
        self.fork()
            .start_command(profile)
            .with_arg("auth")
            .with_arg("login")
    }

    fn session_sources(&self) -> Vec<Box<dyn SessionSource>> {
        let sessions = DatabaseSessions::new(Arc::clone(&self.location));

        vec![Box::new(sessions)]
    }

    fn session_source_ids(&self) -> &'static [SourceId] {
        &SESSION_SOURCES
    }

    /// None: OTTY reads no limits through OpenCode or Kilo. The limits of
    /// a ChatGPT login they share with Codex are read through Codex.
    fn limit_sources(
        &self,
        _programs: Arc<dyn StartProgram>,
        _terminals: Arc<dyn StartTerminal>,
    ) -> Vec<Box<dyn LimitSource>> {
        Vec::new()
    }

    fn limit_source_ids(&self) -> &'static [SourceId] {
        &[]
    }

    /// The tokens and the cost of the answers in the database.
    fn spend_counter(&self) -> Option<Box<dyn SpendCounter>> {
        let counter = DatabaseSpend::new(Arc::clone(&self.location));

        Some(Box::new(counter))
    }

    /// None: the money is the cost the agent wrote for each answer.
    fn bill_source(&self) -> Option<Box<dyn BillSource>> {
        None
    }

    fn stats_source(&self) -> Option<Box<dyn StatsSource>> {
        let stats = DatabaseStats::new(Arc::clone(&self.location));

        Some(Box::new(stats))
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::open_code::testing::MadeUpDatabase;
    use crate::testing::{NOW, TempDir, no_programs, no_terminals, write_file};

    const ID: &str = "ses_2f1c0a8e7ffeL0q3mRk9ZbXw2a";

    fn default_kilo() -> Profile {
        Profile::default_for(
            KILO.id(),
            "Kilo",
            PathBuf::from("/home/user/.local/share/kilo"),
        )
    }

    fn added(fork: &Fork, folder: &Path) -> Profile {
        Profile::added(fork.id(), "Test", folder.to_path_buf())
    }

    #[test]
    fn the_default_profile_is_the_data_folder() {
        let env = AgentEnv::new(PathBuf::from("/home/user"));

        assert_eq!(
            OpenCodeAgent::kilo().default_config_dir(&env),
            PathBuf::from("/home/user/.local/share/kilo")
        );
        assert_eq!(
            OpenCodeAgent::opencode().default_config_dir(&env),
            PathBuf::from("/home/user/.local/share/opencode")
        );
    }

    #[test]
    fn a_session_is_continued_with_its_id_and_a_login_runs_auth_login() {
        let kilo = OpenCodeAgent::kilo();
        let opencode = OpenCodeAgent::opencode();
        let id = SessionId::new(ID);

        assert_eq!(
            kilo.launch(&default_kilo(), None),
            AgentCommand::new("kilo")
        );
        assert_eq!(
            kilo.launch(&default_kilo(), Some(&id)),
            AgentCommand::new("kilo").with_arg("-s").with_arg(ID)
        );
        assert_eq!(
            kilo.login(&default_kilo()),
            AgentCommand::new("kilo").with_arg("auth").with_arg("login")
        );
        let work = added(&OPENCODE, Path::new("/home/user/work/opencode"));
        assert_eq!(
            opencode.launch(&work, Some(&id)),
            AgentCommand::new("opencode")
                .with_arg("-s")
                .with_arg(ID)
                .with_env("XDG_DATA_HOME", "/home/user/work")
        );
    }

    #[test]
    fn a_profile_with_a_login_is_connected_and_a_key_pays_per_token() {
        let temp = TempDir::new();
        let chatgpt = temp.path().join("chatgpt/kilo");
        write_file(
            &chatgpt.join("auth.json"),
            r#"{"openai": {"type": "oauth", "access": "opaque", "refresh": "r",
            "expires": 1, "accountId": "00000000-0000-4000-8000-0000000000c1"}}"#,
        );
        let key = temp.path().join("key/kilo");
        write_file(
            &key.join("auth.json"),
            r#"{"deepseek": {"type": "api", "key": "sk-made-up-key-00000000"}}"#,
        );
        let none = temp.path().join("none/kilo");
        std::fs::create_dir_all(&none).expect("create the data folder");
        let kilo = OpenCodeAgent::kilo();

        let connected = |billing| ProfileStatus::Connected {
            account: None,
            plan: None,
            billing,
        };
        assert_eq!(kilo.detect(&added(&KILO, &chatgpt)), connected(None));
        assert_eq!(
            kilo.detect(&added(&KILO, &key)),
            connected(Some(BillingKind::ApiKey))
        );
        assert_eq!(
            kilo.detect(&added(&KILO, &none)),
            ProfileStatus::NotLoggedIn
        );
        assert_eq!(
            kilo.detect(&added(&KILO, &temp.path().join("gone"))),
            ProfileStatus::ConfigDirNotFound
        );
    }

    #[test]
    fn the_version_and_the_sessions_come_from_the_database() {
        let temp = TempDir::new();
        let data_dir = temp.path().join("kilo");
        let made_up = MadeUpDatabase::first_shape(&data_dir.join("kilo.db"));
        let kilo = OpenCodeAgent::kilo();
        let profile = added(&KILO, &data_dir);

        let before = (kilo.has_sessions(&profile), kilo.version(&profile));
        made_up.session("ses_1", None, "/home/user/otty", "7.5.9", (NOW, NOW));
        made_up.session(
            "ses_2",
            None,
            "/home/user/otty",
            "7.8.27",
            (NOW, NOW + 1),
        );
        let after = (kilo.has_sessions(&profile), kilo.version(&profile));

        assert_eq!(before, (false, None));
        assert_eq!(after, (true, Some(String::from("7.8.27"))));
    }

    #[test]
    fn sessions_tokens_and_statistics_come_from_the_database() {
        let kilo = OpenCodeAgent::kilo();

        let sessions: Vec<_> = kilo
            .session_sources()
            .iter()
            .map(|source| source.id())
            .collect();

        assert_eq!(sessions, [SourceId::AgentDatabase]);
        assert_eq!(sessions, kilo.session_source_ids());
        assert!(kilo.limit_sources(no_programs(), no_terminals()).is_empty());
        assert!(kilo.limit_source_ids().is_empty());
        assert!(kilo.spend_counter().is_some());
        assert!(kilo.stats_source().is_some());
        assert!(kilo.bill_source().is_none());
        assert!(kilo.writes_cost());
    }
}

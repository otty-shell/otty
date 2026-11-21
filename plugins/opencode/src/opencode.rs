//! The agent part `opencode`: OpenCode, the coding agent by Anomaly, and
//! the commands that start it.

use otty_plugin_sdk::agents::{
    Agent, Answers, Login, LoginRef, Place, PlaceItem, Profile, ProfilePlace,
    ProfileStatus, ReadError, Session,
};
use otty_plugin_sdk::{Command, Need};

use crate::database::{Database, database_places};
use crate::location::{database_of, profiles, start_command};
use crate::sessions::newest_version;
use crate::{OpenCode, answers, logins, needs, sessions};

impl Agent for OpenCode {
    /// The default data folder; OpenCode has no other.
    fn profiles(_agent: &str) -> Vec<ProfilePlace> {
        profiles()
    }

    /// Connected with any login it keeps; paid per token when one of
    /// them is a key. Looking for the program on `PATH` is left to OTTY.
    fn detect(profile: &Profile) -> ProfileStatus {
        logins::status(profile)
    }

    /// The version of OpenCode that wrote the session that changed last.
    fn version(profile: &Profile) -> Option<String> {
        let database = Database::open(&database_of(profile)?).ok()??;
        let sessions = database.sessions().ok()?;

        newest_version(&sessions)
    }

    fn logins(profile: &Profile) -> Vec<Login> {
        logins::logins(profile)
    }

    /// The database and its write-ahead log; none while the database is
    /// kept in memory.
    fn session_places(profile: &Profile) -> Vec<Place> {
        database_of(profile)
            .map(|database| database_places(&database))
            .unwrap_or_default()
    }

    fn read_sessions(
        _profile: &Profile,
        item: &PlaceItem,
    ) -> Result<Vec<Session>, ReadError> {
        sessions::read_sessions(item)
    }

    /// The database and its write-ahead log; none while the database is
    /// kept in memory.
    fn spend_places(profile: &Profile) -> Vec<Place> {
        database_of(profile)
            .map(|database| database_places(&database))
            .unwrap_or_default()
    }

    fn read_answers(
        _profile: &Profile,
        item: &PlaceItem,
    ) -> Result<Answers, ReadError> {
        answers::read_answers(item)
    }

    /// `opencode` for a new session, `opencode -s <id>` to continue one.
    fn launch(profile: &Profile, session: Option<&str>) -> Command {
        let args = match session {
            Some(session) => vec!["-s".to_string(), session.to_string()],
            None => Vec::new(),
        };

        start_command(profile, args)
    }

    /// A provider refused a login OpenCode keeps, such as a ChatGPT login
    /// the `openai` provider part read: OpenCode logs in again with
    /// `opencode auth login`, in the profile's data folder. Never Codex:
    /// the login is OpenCode's.
    fn login_refused(profile: &Profile, _login: &LoginRef) -> Option<Need> {
        let login = start_command(
            profile,
            vec!["auth".to_string(), "login".to_string()],
        );

        Some(needs::login_refused(login))
    }
}

#[cfg(test)]
mod tests {
    use otty_plugin_sdk::agents::{
        Billing, Connection, LoginKind, PlaceKind, ProviderRef,
    };

    use super::*;
    use crate::location::DATABASE_VARIABLE;
    use crate::testing::{AGENT, NOW, as_part, host, millis, path};

    /// The ChatGPT session of the fixtures, in `/home/dev/shop`.
    const SHOP_SESSION: &str = "ses_shop";

    fn default_profile(host: &otty_plugin_sdk::test_host::TestHost) -> Profile {
        Profile {
            agent: AGENT.to_string(),
            folder: path(host, ".local/share/opencode"),
            is_default: true,
        }
    }

    fn command(args: &[&str]) -> Command {
        Command {
            program: "opencode".to_string(),
            args: args.iter().map(|arg| arg.to_string()).collect(),
            env: Vec::new(),
        }
    }

    /// Issue 20: a ChatGPT login OpenCode keeps, refused by the `openai`
    /// provider part, is logged in again with OpenCode, not Codex.
    #[test]
    fn a_refused_login_logs_in_again_with_opencode() {
        let host = host();
        let profile = default_profile(&host);
        let login = LoginRef::EnvVar("OPENAI_API_KEY".to_string());

        let need = as_part(&host, || OpenCode::login_refused(&profile, &login))
            .expect("a need");

        assert_eq!(need.id, "login-refused");
        assert_eq!(
            need.action,
            otty_plugin_sdk::NeedAction::Run(command(&["auth", "login"]))
        );
    }

    #[test]
    fn a_session_is_continued_with_its_id() {
        let host = host();
        let profile = default_profile(&host);

        assert_eq!(OpenCode::launch(&profile, None), command(&[]));
        assert_eq!(
            OpenCode::launch(&profile, Some(SHOP_SESSION)),
            command(&["-s", SHOP_SESSION])
        );
    }

    /// The fixture data folder: a ChatGPT login in `auth.json`, a DeepSeek
    /// key in the `credential` table, two sessions and their answers.
    #[test]
    fn the_fixture_folder_is_a_connected_profile_with_its_logins() {
        let host = host();
        let profile = default_profile(&host);

        let (places, status, logins) = as_part(&host, || {
            (
                OpenCode::profiles(AGENT),
                OpenCode::detect(&profile),
                OpenCode::logins(&profile),
            )
        });

        assert_eq!(
            places,
            [ProfilePlace {
                folder: profile.folder.clone(),
                is_default: true,
            }]
        );
        assert_eq!(
            status,
            ProfileStatus::Connected(Connection {
                account: None,
                plan: None,
                billing: Some(Billing::ApiKey),
            })
        );
        let kinds: Vec<(LoginKind, ProviderRef)> = logins
            .into_iter()
            .map(|login| (login.kind, login.for_whom))
            .collect();
        assert_eq!(
            kinds,
            [
                (
                    LoginKind::Subscription,
                    ProviderRef::Id("openai".to_string())
                ),
                (LoginKind::ApiKey, ProviderRef::Id("deepseek".to_string())),
            ]
        );
    }

    #[test]
    fn the_fixture_folder_has_its_sessions_and_version() {
        let host = host();
        let profile = default_profile(&host);

        let items =
            host.place_items(AGENT, &OpenCode::session_places(&profile));
        let (sessions, version) = as_part(&host, || {
            (
                OpenCode::read_sessions(&profile, &items[0])
                    .expect("the sessions"),
                OpenCode::version(&profile),
            )
        });

        let mut ids: Vec<&str> =
            sessions.iter().map(|session| session.id.as_str()).collect();
        ids.sort();
        assert_eq!(items.len(), 1, "the fixtures keep no write-ahead log");
        assert!(items.iter().all(|item| item.kind == PlaceKind::Database));
        assert_eq!(ids, ["ses_blog", SHOP_SESSION]);
        assert_eq!(version.as_deref(), Some("2.3.2"));
    }

    /// The host walks the spend places, the database and its log, and
    /// asks for the answers of what changed: every answer of the history,
    /// the old one too, each naming its session.
    #[test]
    fn the_answers_of_the_fixture_database_are_read() {
        let host = host().with_now_ms(millis(NOW));
        let profile = default_profile(&host);

        let items = host.place_items(AGENT, &OpenCode::spend_places(&profile));
        let answers = as_part(&host, || {
            OpenCode::read_answers(&profile, &items[0]).expect("the answers")
        });

        assert_eq!(answers.answers.len(), 3);
        assert!(
            answers.answers.iter().any(|answer| {
                answer.session.as_deref() == Some(SHOP_SESSION)
            })
        );
        assert!(
            answers
                .answers
                .iter()
                .all(|answer| answer.cost_microdollars.is_some())
        );
        assert_eq!((answers.records.seen, answers.records.understood), (3, 3));
    }

    /// A database kept in memory has no file: nothing to read.
    #[test]
    fn a_database_kept_in_memory_has_nothing_to_read() {
        let host = host().with_var(DATABASE_VARIABLE, ":memory:");
        let profile = default_profile(&host);

        let (places, session_places) = as_part(&host, || {
            (
                OpenCode::spend_places(&profile),
                OpenCode::session_places(&profile),
            )
        });

        assert!(places.is_empty());
        assert!(session_places.is_empty());
    }
}

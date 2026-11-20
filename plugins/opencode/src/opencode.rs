//! The agent part `opencode`: OpenCode, the coding agent by Anomaly, and
//! the commands that start it.

use otty_plugin_sdk::{
    Agent, Answers, Command, HostError, Login, Profile, ProfilePlace,
    ProfileStatus, ProjectStats, ReadError, Session, SessionStats, SpendItem,
    SpendPlace,
};

use crate::database::Database;
use crate::location::{database_of, profiles, start_command};
use crate::sessions::newest_version;
use crate::{OpenCode, answers, logins, sessions, stats};

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

    /// The sessions in the database; none while the database is kept in
    /// memory.
    fn sessions(profile: &Profile) -> Result<Vec<Session>, ReadError> {
        match database_of(profile) {
            Some(database) => sessions::sessions(&database),
            None => Ok(Vec::new()),
        }
    }

    /// The database and its write-ahead log; none while the database is
    /// kept in memory.
    fn spend_places(profile: &Profile) -> Vec<SpendPlace> {
        database_of(profile)
            .map(|database| answers::spend_places(&database))
            .unwrap_or_default()
    }

    fn read_answers(
        _profile: &Profile,
        item: &SpendItem,
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

    /// `opencode auth login`.
    fn login_command(profile: &Profile) -> Command {
        start_command(profile, vec!["auth".to_string(), "login".to_string()])
    }

    fn read_session_stats(
        profile: &Profile,
        session: &str,
    ) -> Result<SessionStats, ReadError> {
        let database = database_of(profile).ok_or_else(|| {
            ReadError::Host(HostError::NotFound(format!(
                "the session {session}"
            )))
        })?;

        stats::session_stats(&database, session)
    }

    fn read_project_stats(
        profile: &Profile,
        folder: &str,
    ) -> Result<ProjectStats, ReadError> {
        match database_of(profile) {
            Some(database) => stats::project_stats(&database, folder),
            None => Ok(ProjectStats {
                first_ms: None,
                last_ms: None,
                models: Vec::new(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use otty_plugin_sdk::{
        Billing, Connection, LoginKind, ProviderRef, SpendKind, files,
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

    #[test]
    fn a_session_is_continued_with_its_id_and_a_login_runs_auth_login() {
        let host = host();
        let profile = default_profile(&host);

        assert_eq!(OpenCode::launch(&profile, None), command(&[]));
        assert_eq!(
            OpenCode::launch(&profile, Some(SHOP_SESSION)),
            command(&["-s", SHOP_SESSION])
        );
        assert_eq!(
            OpenCode::login_command(&profile),
            command(&["auth", "login"])
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
    fn the_fixture_folder_has_its_sessions_version_and_statistics() {
        let host = host();
        let profile = default_profile(&host);

        let (sessions, version, session, project) = as_part(&host, || {
            (
                OpenCode::sessions(&profile).expect("the sessions"),
                OpenCode::version(&profile),
                OpenCode::read_session_stats(&profile, SHOP_SESSION)
                    .expect("the session"),
                OpenCode::read_project_stats(&profile, "/home/dev/shop")
                    .expect("the project"),
            )
        });

        let ids: Vec<&str> =
            sessions.iter().map(|session| session.id.as_str()).collect();
        assert_eq!(ids, ["ses_blog", SHOP_SESSION]);
        assert_eq!(version.as_deref(), Some("2.3.2"));
        assert_eq!(session.messages, 1);
        assert_eq!(session.models[0].model, "gpt-5.5");
        assert_eq!(project.models[0].tokens.input, 1210);
    }

    /// The host walks the spend places, the database and its log, and
    /// asks for the answers of what changed.
    #[test]
    fn the_answers_of_the_fixture_database_are_read() {
        let host = host().with_now_ms(millis(NOW));
        let profile = default_profile(&host);

        let answers = as_part(&host, || {
            let found: Vec<String> = OpenCode::spend_places(&profile)
                .iter()
                .flat_map(|place| {
                    files::list(&place.glob).expect("the listing")
                })
                .map(|file| file.path)
                .collect();
            let item = SpendItem {
                path: found[0].clone(),
                kind: SpendKind::Database,
            };

            OpenCode::read_answers(&profile, &item).expect("the answers")
        });

        assert_eq!(answers.answers.len(), 2);
        assert!(
            answers
                .answers
                .iter()
                .all(|answer| answer.cost_microdollars.is_some())
        );
        assert_eq!((answers.records.seen, answers.records.understood), (2, 2));
    }

    /// A database kept in memory has no file: nothing to read.
    #[test]
    fn a_database_kept_in_memory_has_nothing_to_read() {
        let host = host().with_var(DATABASE_VARIABLE, ":memory:");
        let profile = default_profile(&host);

        let (places, sessions, project) = as_part(&host, || {
            (
                OpenCode::spend_places(&profile),
                OpenCode::sessions(&profile),
                OpenCode::read_project_stats(&profile, "/home/dev/shop"),
            )
        });

        assert!(places.is_empty());
        assert_eq!(sessions, Ok(Vec::new()));
        assert_eq!(project.map(|stats| stats.models.len()), Ok(0));
    }
}

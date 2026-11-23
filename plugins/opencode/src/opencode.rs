//! The agent part `opencode`: OpenCode, the coding agent by Anomaly, and
//! the commands that start it.

use otty_plugin_sdk::Command;
use otty_plugin_sdk::agents::{
    Agent, Answers, Place, PlaceItem, Profile, ProfilePlace, ProfileStatus,
    ReadError, Session,
};

use crate::database::{Database, database_places};
use crate::location::{database_of, profiles, start_command};
use crate::sessions::newest_version;
use crate::{OpenCode, answers, sessions};

impl Agent for OpenCode {
    /// The default data folder; OpenCode has no other.
    fn profiles(_agent: &str) -> Vec<ProfilePlace> {
        profiles()
    }

    /// Always found: OpenCode works with no login, with its free models,
    /// so the part never says a login is missing, not even for an empty
    /// data folder, as a new profile is (spec
    /// 2026-11-23-accounts-apart-from-profiles, Decision 9, issue 12).
    /// Looking for the program on `PATH` and for the folder is left to
    /// OTTY.
    fn detect(_profile: &Profile) -> ProfileStatus {
        ProfileStatus::Found
    }

    /// The version of OpenCode that wrote the session that changed last.
    fn version(profile: &Profile) -> Option<String> {
        let database = Database::open(&database_of(profile)?).ok()??;
        let sessions = database.sessions().ok()?;

        newest_version(&sessions)
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
}

#[cfg(test)]
mod tests {
    use otty_plugin_sdk::agents::{AnswerCost, PlaceKind};

    use super::*;
    use crate::location::DATABASE_VARIABLE;
    use crate::testing::{AGENT, DataFolder, NOW, as_part, host, millis, path};

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
    fn a_session_is_continued_with_its_id() {
        let host = host();
        let profile = default_profile(&host);

        assert_eq!(OpenCode::launch(&profile, None), command(&[]));
        assert_eq!(
            OpenCode::launch(&profile, Some(SHOP_SESSION)),
            command(&["-s", SHOP_SESSION])
        );
    }

    /// Spec 2026-11-23-accounts-apart-from-profiles, Decision 11: OpenCode
    /// is an agent only. The fixture data folder keeps a ChatGPT login and
    /// a DeepSeek key, and the profile is found with no word on them.
    #[test]
    fn the_fixture_folder_is_a_found_profile() {
        let host = host();
        let profile = default_profile(&host);

        let (places, status) = as_part(&host, || {
            (OpenCode::profiles(AGENT), OpenCode::detect(&profile))
        });

        assert_eq!(
            places,
            [ProfilePlace {
                folder: profile.folder.clone(),
                is_default: true,
            }]
        );
        assert_eq!(status, ProfileStatus::Found);
    }

    /// Decision 9: OpenCode works with no login, so a data folder with no
    /// login is found, and so is an empty one, as a new profile is, with
    /// no need (issue 12 of spec 2026-11-23-accounts-apart-from-profiles).
    /// OTTY tells a folder that is not there before it asks the part.
    #[test]
    fn a_folder_with_no_login_and_an_empty_one_are_found() {
        let no_login = DataFolder::new();
        let _database = no_login.first_shape();
        let empty = DataFolder::new();

        let no_login_status =
            no_login.run(|| OpenCode::detect(&no_login.profile()));
        let empty_status = empty.run(|| OpenCode::detect(&empty.profile()));

        assert_eq!(no_login_status, ProfileStatus::Found);
        assert_eq!(empty_status, ProfileStatus::Found);
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
    /// the old one too, each naming its session, with the cost OpenCode
    /// wrote, exact (Decision 3).
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
            answers.answers.iter().all(|answer| matches!(
                answer.cost,
                Some(AnswerCost::Written(_))
            ))
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

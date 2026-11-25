//! The agent part `example-agent`: a made-up coding agent that keeps its
//! login in `auth.json`, its version in `state.json` and its sessions in
//! `sessions/*.jsonl` of its config folder, `~/.example-agent` by default.
//! The part does agent things only: whether the agent can log in with a
//! profile, never whose login it is, which is the provider part's
//! business.

use std::path::Path;

use otty_plugin_sdk::agents::{
    Agent, Answers, Place, PlaceItem, PlaceKind, Profile, ProfilePlace,
    ProfileStatus, ReadError, Session,
};
use otty_plugin_sdk::{Command, HostError, env, files, log};
use serde::Deserialize;

use crate::session_file::SessionFile;
use crate::{Example, needs};

/// The variable that names another config folder, as the agent reads it.
pub(crate) const HOME_VARIABLE: &str = "EXAMPLE_AGENT_HOME";

/// The agent's default config folder in the home folder.
const DEFAULT_FOLDER: &str = ".example-agent";

/// The agent's program.
const PROGRAM: &str = "example-agent";

/// The most bytes of a small file the part reads at once.
const SMALL_FILE: u32 = 64 * 1024;

/// `auth.json`, its key replaced by a handle: the part only tells whether
/// a key is there.
#[derive(Debug, Deserialize)]
struct AuthFile {
    api_key: Option<KeyMarker>,
}

/// `{"$key": n}`: where the host put a key.
#[derive(Debug, Deserialize)]
struct KeyMarker {
    #[serde(rename = "$key")]
    _index: usize,
}

/// `state.json`.
#[derive(Debug, Deserialize)]
struct StateFile {
    last_version: Option<String>,
}

impl Agent for Example {
    /// The default config folder only: OTTY ignores any other place, and
    /// the user adds other folders as profiles.
    fn profiles(_agent: &str) -> Vec<ProfilePlace> {
        let folder = env::var(HOME_VARIABLE)
            .unwrap_or_else(|| format!("{}/{DEFAULT_FOLDER}", env::home()));

        vec![ProfilePlace {
            folder,
            is_default: true,
        }]
    }

    /// Found with a key in `auth.json`, else not logged in with the need
    /// that runs the agent's login in the profile's folder: whether the
    /// agent can log in is this part's call. OTTY asks only about a folder
    /// that is there, so an empty one, as a new profile is, has no
    /// `auth.json` and needs the login. An agent that works with no login
    /// would say found there.
    fn detect(profile: &Profile) -> ProfileStatus {
        match read_auth(profile) {
            Ok(AuthFile { api_key: Some(_) }) => ProfileStatus::Found,
            _ => ProfileStatus::NotLoggedIn(needs::not_logged_in(login_run(
                profile,
            ))),
        }
    }

    fn version(profile: &Profile) -> Option<String> {
        let path = format!("{}/state.json", profile.folder);
        let bytes = files::read(&path, SMALL_FILE).ok()?;
        let Ok(state) = serde_json::from_slice::<StateFile>(&bytes) else {
            log::warn("state.json is not in the shape this plugin reads");
            return None;
        };

        state.last_version
    }

    /// Each session file is one session; OTTY reads again only the files
    /// that changed.
    fn session_places(profile: &Profile) -> Vec<Place> {
        vec![Place {
            glob: sessions_glob(profile),
            kind: PlaceKind::Files,
        }]
    }

    fn read_sessions(
        _profile: &Profile,
        item: &PlaceItem,
    ) -> Result<Vec<Session>, ReadError> {
        let session = SessionFile::read(&item.file.path)?;

        Ok(vec![Session {
            id: session_id(&item.file.path),
            folder: session.project().map(str::to_string),
            title: session.title().map(str::to_string),
            changed_ms: session.last_at_ms().unwrap_or(item.file.changed_ms),
        }])
    }

    fn spend_places(profile: &Profile) -> Vec<Place> {
        vec![Place {
            glob: sessions_glob(profile),
            kind: PlaceKind::Files,
        }]
    }

    /// Each answer names its session, the file's: OTTY sums the panel of
    /// one session from the answers that name it. Each answer carries its
    /// money: the cost the agent wrote, else its tokens at the price
    /// OTTY's price list gives.
    fn read_answers(
        _profile: &Profile,
        item: &PlaceItem,
    ) -> Result<Answers, ReadError> {
        let session = session_id(&item.file.path);

        SessionFile::read(&item.file.path)
            .map(|file| file.into_answers(&session))
    }

    fn launch(profile: &Profile, session: Option<&str>) -> Command {
        let args = match session {
            Some(session) => vec!["--resume".to_string(), session.to_string()],
            None => Vec::new(),
        };

        Command {
            program: PROGRAM.to_string(),
            args,
            env: folder_env(profile),
        }
    }
}

/// `example-agent login` pointed at `folder`, which is never the default
/// one: how the provider part logs in to OTTY's own login folder.
pub(crate) fn login_in(folder: &str) -> Command {
    Command {
        program: PROGRAM.to_string(),
        args: vec!["login".to_string()],
        env: vec![(HOME_VARIABLE.to_string(), folder.to_string())],
    }
}

/// The command that logs the profile in: `example-agent login`.
fn login_run(profile: &Profile) -> Command {
    Command {
        program: PROGRAM.to_string(),
        args: vec!["login".to_string()],
        env: folder_env(profile),
    }
}

/// `auth.json` of the profile, its key replaced by a handle.
fn read_auth(profile: &Profile) -> Result<AuthFile, HostError> {
    let document = files::json_with_keys(&auth_path(profile))?;

    serde_json::from_str(document.json()).map_err(|_| {
        HostError::Failed("auth.json is not an auth file".to_string())
    })
}

/// The id of the session kept in the session file at `path`: its name
/// without `.jsonl`.
fn session_id(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// The session files of the profile: `sessions/*.jsonl`.
fn sessions_glob(profile: &Profile) -> String {
    format!("{}/sessions/*.jsonl", profile.folder)
}

fn auth_path(profile: &Profile) -> String {
    format!("{}/auth.json", profile.folder)
}

/// The variable a profile in another folder than the default needs.
fn folder_env(profile: &Profile) -> Vec<(String, String)> {
    if profile.is_default {
        return Vec::new();
    }

    vec![(HOME_VARIABLE.to_string(), profile.folder.clone())]
}

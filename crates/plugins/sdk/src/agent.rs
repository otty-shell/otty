//! The agent contract `otty:agent@0.1.0` as a trait.

use std::marker::PhantomData;

use crate::bindings::agent::exports::otty::agent::agent::Guest;
use crate::{
    Answers, Command, Limits, Login, LoginRef, Profile, ProfilePlace,
    ProfileStatus, ProjectStats, ReadError, Session, SessionStats, SpendItem,
    SpendPlace,
};

/// One coding agent program: its profiles, sessions, tokens and logins.
/// A plugin implements it once and serves each agent part its manifest
/// lists; `agent` or `profile.agent` says which part is asked.
///
/// The functions with a body are optional: their default says "not
/// supported". The rule of the spec: what breaks when the agent updates
/// (field names, file names, JSON shapes, SQL) lives here, never in OTTY.
pub trait Agent {
    /// The config folders of the agent part `agent` found on this
    /// computer, default first: the default folder (or the one its
    /// variable names) and extra folders, such as `~/.codex-work`, that
    /// hold a login or a session.
    fn profiles(agent: &str) -> Vec<ProfilePlace>;

    /// Whether the profile is logged in, or needs a login.
    fn detect(profile: &Profile) -> ProfileStatus;

    /// The agent's version written in the profile's newest files. `None`:
    /// OTTY runs `<program> --version` itself.
    fn version(profile: &Profile) -> Option<String>;

    /// The logins and keys the profile holds, each with where it sits and
    /// whom it is for. Never the key itself.
    fn logins(profile: &Profile) -> Vec<Login>;

    /// The sessions of the profile, newest first.
    fn sessions(profile: &Profile) -> Result<Vec<Session>, ReadError>;

    /// What holds the profile's answers. The host walks the files and asks
    /// again only for the ones that changed.
    fn spend_places(profile: &Profile) -> Vec<SpendPlace>;

    /// Every answer of the model in one file or database of a spend place,
    /// with the counts of records seen and understood.
    fn read_answers(
        profile: &Profile,
        item: &SpendItem,
    ) -> Result<Answers, ReadError>;

    /// The command that starts a new session, or continues `session`.
    fn launch(profile: &Profile, session: Option<&str>) -> Command;

    /// The command that logs the profile in.
    fn login_command(profile: &Profile) -> Command;

    /// The sources of limits the agent itself shows for a login of the
    /// profile, best first, each named as the user reads it in "Where the
    /// numbers come from", such as `codex app-server`; the name is also
    /// what [`Agent::read_limits`] gets. Default: none.
    fn limit_sources(_profile: &Profile) -> Vec<String> {
        Vec::new()
    }

    /// The limits of `login` from one of those sources. Default: not
    /// supported.
    fn read_limits(
        _profile: &Profile,
        _login: &LoginRef,
        _source: &str,
    ) -> Result<Limits, ReadError> {
        Err(ReadError::NotSupported)
    }

    /// What the files of one session tell. Default: not supported.
    fn read_session_stats(
        _profile: &Profile,
        _session: &str,
    ) -> Result<SessionStats, ReadError> {
        Err(ReadError::NotSupported)
    }

    /// What the files of one project folder tell. Default: not supported.
    fn read_project_stats(
        _profile: &Profile,
        _folder: &str,
    ) -> Result<ProjectStats, ReadError> {
        Err(ReadError::NotSupported)
    }
}

/// What [`export!`](crate::export) exports for a plugin type `T` with
/// `agent`: the binding's trait, served by `T`'s [`Agent`].
#[doc(hidden)]
pub struct AgentExport<T>(PhantomData<T>);

impl<T: Agent> Guest for AgentExport<T> {
    fn profiles(agent: String) -> Vec<ProfilePlace> {
        T::profiles(&agent)
    }

    fn detect(profile: Profile) -> ProfileStatus {
        T::detect(&profile)
    }

    fn version(profile: Profile) -> Option<String> {
        T::version(&profile)
    }

    fn logins(profile: Profile) -> Vec<Login> {
        T::logins(&profile)
    }

    fn sessions(profile: Profile) -> Result<Vec<Session>, ReadError> {
        T::sessions(&profile)
    }

    fn spend_places(profile: Profile) -> Vec<SpendPlace> {
        T::spend_places(&profile)
    }

    fn read_answers(
        profile: Profile,
        item: SpendItem,
    ) -> Result<Answers, ReadError> {
        T::read_answers(&profile, &item)
    }

    fn launch(profile: Profile, session: Option<String>) -> Command {
        T::launch(&profile, session.as_deref())
    }

    fn login_command(profile: Profile) -> Command {
        T::login_command(&profile)
    }

    fn limit_sources(profile: Profile) -> Vec<String> {
        T::limit_sources(&profile)
    }

    fn read_limits(
        profile: Profile,
        login: LoginRef,
        source: String,
    ) -> Result<Limits, ReadError> {
        T::read_limits(&profile, &login, &source)
    }

    fn read_session_stats(
        profile: Profile,
        session: String,
    ) -> Result<SessionStats, ReadError> {
        T::read_session_stats(&profile, &session)
    }

    fn read_project_stats(
        profile: Profile,
        folder: String,
    ) -> Result<ProjectStats, ReadError> {
        T::read_project_stats(&profile, &folder)
    }
}

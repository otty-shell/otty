//! The agent contract `otty:agent@0.1.0` as a trait.

use std::marker::PhantomData;

use crate::bindings::agent::exports::otty::agent::agent::Guest;
use crate::{
    Answers, Command, Limits, Login, LoginRef, Need, Profile, ProfilePlace,
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

    /// Whether the profile is logged in. A profile nobody is logged in
    /// with is [`ProfileStatus::NotLoggedIn`] with the need that says how
    /// to log in, such as a [`NeedAction::Run`](crate::NeedAction) of the
    /// agent's login command.
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

    /// The limits the agent itself shows for `login` of the profile. Where
    /// they come from is the plugin's business: it tries its own programs,
    /// files or servers in its own order and gives the first limits it
    /// gets, or one error. Default: not supported, and OTTY shows no
    /// limits of the agent.
    fn read_limits(
        _profile: &Profile,
        _login: &LoginRef,
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

    /// What the user must do now that a provider part refused `login` of
    /// the profile, or saw it expired
    /// ([`ReadError::LoginRefused`]): the provider part does not know
    /// which agent keeps the login, so OTTY asks the agent part that
    /// found it, such as a need that runs the agent in the profile's
    /// folder so it renews its login. Default: `None`, and OTTY says the
    /// login was refused, with no button.
    fn login_refused(_profile: &Profile, _login: &LoginRef) -> Option<Need> {
        None
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

    fn read_limits(
        profile: Profile,
        login: LoginRef,
    ) -> Result<Limits, ReadError> {
        T::read_limits(&profile, &login)
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

    fn login_refused(profile: Profile, login: LoginRef) -> Option<Need> {
        T::login_refused(&profile, &login)
    }
}

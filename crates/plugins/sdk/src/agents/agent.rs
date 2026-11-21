//! The agent contract `otty:agents-agent@0.1.0` as a trait.

use std::marker::PhantomData;

use crate::agents::{
    Answers, Limits, Login, LoginRef, Place, PlaceItem, Profile, ProfilePlace,
    ProfileStatus, ReadError, Session,
};
use crate::bindings::agents_agent::exports::otty::agents_agent::agent::Guest;
use crate::{Command, Need};

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

    /// What holds the profile's sessions. The host walks the files, keeps
    /// what each gave and asks again only for the ones that are new or
    /// changed, so the part keeps nothing between reads.
    fn session_places(profile: &Profile) -> Vec<Place>;

    /// What one file or database of a session place tells about its
    /// sessions. The host joins what several files tell about one id: the
    /// latest time, and the folder and the title of the latest record
    /// that names them, so a file may name only some of them, such as an
    /// index of titles. A session no file names a folder for is not
    /// listed.
    fn read_sessions(
        profile: &Profile,
        item: &PlaceItem,
    ) -> Result<Vec<Session>, ReadError>;

    /// What holds the profile's answers. The host walks the files and asks
    /// again only for the ones that changed.
    fn spend_places(profile: &Profile) -> Vec<Place>;

    /// Every answer of the model in one file or database of a spend place,
    /// with the counts of records seen and understood.
    fn read_answers(
        profile: &Profile,
        item: &PlaceItem,
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
/// `agents::agent`: the binding's trait, served by `T`'s [`Agent`].
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

    fn session_places(profile: Profile) -> Vec<Place> {
        T::session_places(&profile)
    }

    fn read_sessions(
        profile: Profile,
        item: PlaceItem,
    ) -> Result<Vec<Session>, ReadError> {
        T::read_sessions(&profile, &item)
    }

    fn spend_places(profile: Profile) -> Vec<Place> {
        T::spend_places(&profile)
    }

    fn read_answers(
        profile: Profile,
        item: PlaceItem,
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

    fn login_refused(profile: Profile, login: LoginRef) -> Option<Need> {
        T::login_refused(&profile, &login)
    }
}

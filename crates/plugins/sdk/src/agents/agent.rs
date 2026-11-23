//! The agent contract `otty:agents-agent@0.1.0` as a trait.

use std::marker::PhantomData;

use crate::Command;
use crate::agents::{
    Answers, Place, PlaceItem, Profile, ProfilePlace, ProfileStatus, ReadError,
    Session,
};
use crate::bindings::agents_agent::exports::otty::agents_agent::agent::Guest;

/// One coding agent program: its profiles, sessions, tokens and their
/// money. A plugin implements it once and serves each agent part its
/// manifest lists; `agent` or `profile.agent` says which part is asked.
///
/// An agent part knows nothing of accounts (spec
/// 2026-11-23-accounts-apart-from-profiles, Decision 2): whose login a
/// profile holds is the business of the plugin's provider part.
///
/// The rule of the spec: what breaks when the agent updates (field
/// names, file names, JSON shapes, SQL) lives here, never in OTTY.
pub trait Agent {
    /// The default config folder of the agent part `agent` only, marked
    /// `is_default`: the folder the agent uses with no variable set, or
    /// the one its variable names. OTTY ignores any other place; the user
    /// adds other folders, such as `~/.codex-work`, as profiles.
    fn profiles(agent: &str) -> Vec<ProfilePlace>;

    /// Whether the profile can be used: [`ProfileStatus::Found`], or
    /// [`ProfileStatus::NotLoggedIn`] with the need that says how to log
    /// in, such as a [`NeedAction::Run`](crate::NeedAction) of the agent's
    /// login command, when the agent can not log in with it. Whether a
    /// login is missing or expired is the part's own call; an agent that
    /// works with no login is always found.
    fn detect(profile: &Profile) -> ProfileStatus;

    /// The agent's version written in the profile's newest files. `None`:
    /// OTTY runs `<program> --version` itself.
    fn version(profile: &Profile) -> Option<String>;

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
    /// with the counts of records seen and understood. Each answer carries
    /// its money: the cost the agent wrote, else its tokens at the API
    /// price OTTY gives, as [`answer_cost`](crate::agents::answer_cost)
    /// makes it.
    fn read_answers(
        profile: &Profile,
        item: &PlaceItem,
    ) -> Result<Answers, ReadError>;

    /// The command that starts a new session, or continues `session`.
    fn launch(profile: &Profile, session: Option<&str>) -> Command;
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
}

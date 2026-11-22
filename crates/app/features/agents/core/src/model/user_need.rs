//! What a profile that can not be used needs from the user, in a few words
//! for its row in the left panel: to log in, the agent's program, or its
//! config folder (spec Decision 6). How to log in is the agent part's
//! need (spec 2026-11-19-agent-plugins, issue 20); OTTY tells only what
//! it finds missing itself.

use otty_agents::ProfileStatus;
use otty_app_i18n::{self as i18n, Key};

/// What a profile that can not be used needs from the user, in a few
/// words for its row in the left panel and, for what OTTY finds missing
/// itself, a hint for its page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct UserNeed {
    state: &'static str,
    hint: Option<&'static str>,
}

impl UserNeed {
    /// The need in a few words, such as "needs login".
    pub(super) fn state(&self) -> &'static str {
        self.state
    }

    /// What is missing, for the profile page; `None` for a login, whose
    /// need its agent part gives, and for a program, whose need names
    /// where OTTY looked (see `needs.rs`).
    pub(super) fn hint(&self) -> Option<&'static str> {
        self.hint
    }

    /// What a profile with `status` needs; `None` for a connected one.
    pub(super) fn of(status: &ProfileStatus) -> Option<Self> {
        let (state, hint) = match status {
            ProfileStatus::NotLoggedIn(_) => (Key::AgentsStateNeedsLogin, None),
            ProfileStatus::ProgramNotFound => {
                (Key::AgentsStateProgramNotFound, None)
            },
            ProfileStatus::ConfigDirNotFound => (
                Key::AgentsStateConfigDirNotFound,
                Some(Key::AgentsConfigDirNotFoundHint),
            ),
            _ => return None,
        };

        Some(Self {
            state: i18n::t(state),
            hint: hint.map(i18n::t),
        })
    }
}

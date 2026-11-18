//! What a profile that can not be used needs from the user: to log in, the
//! agent's program, or its config folder (spec Decision 6).

use otty_agents::ProfileStatus;
use otty_app_i18n::{self as i18n, Key};

/// What a profile that can not be used needs from the user, in a few
/// words for its row in the left panel and as the hint of its dashboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct UserNeed {
    state: &'static str,
    hint: &'static str,
}

impl UserNeed {
    /// The need in a few words, such as "needs login".
    pub(super) fn state(&self) -> &'static str {
        self.state
    }

    /// What is missing, or what to do, for the dashboard.
    pub(super) fn hint(&self) -> &'static str {
        self.hint
    }

    /// What a profile with `status` needs; `None` for a connected one.
    pub(super) fn of(status: &ProfileStatus) -> Option<Self> {
        let (state, hint) = match status {
            ProfileStatus::NotLoggedIn => {
                (Key::AgentsStateNeedsLogin, Key::AgentsNotLoggedInHint)
            },
            ProfileStatus::ProgramNotFound => (
                Key::AgentsStateProgramNotFound,
                Key::AgentsProgramNotFoundHint,
            ),
            ProfileStatus::ConfigDirNotFound => (
                Key::AgentsStateConfigDirNotFound,
                Key::AgentsConfigDirNotFoundHint,
            ),
            _ => return None,
        };

        Some(Self {
            state: i18n::t(state),
            hint: i18n::t(hint),
        })
    }
}

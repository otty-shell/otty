//! The tabs of the top section of a profile's dashboard: the projects or
//! the models that spent the most tokens in the last 30 days (spec
//! Decision 13).

use std::fmt;

use otty_app_i18n::{self as i18n, Key};

/// What the top section of a dashboard lists. The chosen tab is the same
/// for every profile and is kept while OTTY runs, not saved.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TopTab {
    /// The projects, with their sessions and tokens; chosen at start.
    #[default]
    ByProject,
    /// The models, with their requests and tokens.
    ByModel,
}

impl TopTab {
    /// Both tabs, in the order they show.
    pub(crate) const ALL: [Self; 2] = [Self::ByProject, Self::ByModel];
}

impl fmt::Display for TopTab {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let key = match self {
            Self::ByProject => Key::AgentsTopByProject,
            Self::ByModel => Key::AgentsTopByModel,
        };

        formatter.write_str(i18n::t(key))
    }
}

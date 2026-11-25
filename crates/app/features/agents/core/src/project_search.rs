//! The search field of the projects and the choices of its funnel menu.
//! They live while OTTY runs, not on disk (spec Decision 6).

use std::path::Path;
use std::time::SystemTime;

use otty_agents::Project;

use super::project_filter::{
    ProjectFilter, ProjectFilterChange, filter_projects,
};

/// The search field under the Projects header: whether it is open, the
/// text typed in it and the choices of the funnel menu. Closing the field
/// keeps the text and the choices for the next time it opens; they filter
/// the list only while it is open, so a closed field hides no project.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ProjectSearch {
    is_open: bool,
    query: String,
    filter: ProjectFilter,
}

impl ProjectSearch {
    /// Whether the field shows under the Projects header.
    pub(crate) fn is_open(&self) -> bool {
        self.is_open
    }

    /// The text typed in the field.
    pub(crate) fn query(&self) -> &str {
        &self.query
    }

    /// The choices of the funnel menu.
    pub(crate) fn filter(&self) -> &ProjectFilter {
        &self.filter
    }

    /// Open the field, or close it; return whether it is open now.
    pub(crate) fn toggle(&mut self) -> bool {
        self.is_open = !self.is_open;
        self.is_open
    }

    /// Take the text typed in the field.
    pub(crate) fn set_query(&mut self, query: String) {
        self.query = query;
    }

    /// Esc or the cross: clear the text, or close the field when it has
    /// none.
    pub(crate) fn clear_or_close(&mut self) {
        if self.query.is_empty() {
            self.is_open = false;
        } else {
            self.query.clear();
        }
    }

    /// Take a choice of the funnel menu or the cross of a chip.
    pub(crate) fn change_filter(&mut self, change: ProjectFilterChange) {
        self.filter.apply(change);
    }

    /// The projects the list shows: while the field is open, those that
    /// match its text and choices (see [`filter_projects`]); else all of
    /// them, the latest first.
    pub(crate) fn shown<'a>(
        &self,
        projects: &'a [Project],
        is_missing: impl Fn(&Path) -> bool,
        now: SystemTime,
    ) -> Vec<&'a Project> {
        if !self.is_open {
            return projects.iter().collect();
        }

        filter_projects(projects, &self.query, &self.filter, is_missing, now)
    }
}

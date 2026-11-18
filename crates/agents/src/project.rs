//! Projects: the folders sessions ran in, shared by all profiles (spec
//! Decision 2).

use std::cmp::Reverse;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::profile::Profile;
use crate::session::AgentSession;

/// A folder at least one session ran in, with the sessions of every profile
/// that ran there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    folder: PathBuf,
    changed_at: SystemTime,
    sessions: Vec<ProjectSession>,
}

impl Project {
    /// The project folder.
    pub fn folder(&self) -> &Path {
        &self.folder
    }

    /// The sessions of the project, newest first. Never empty.
    pub fn sessions(&self) -> &[ProjectSession] {
        &self.sessions
    }

    /// When the newest session of the project last changed.
    pub fn changed_at(&self) -> SystemTime {
        self.changed_at
    }

    /// A project from the sessions that ran in `folder`, in any order.
    ///
    /// `None` when there are no sessions: a project always has one.
    fn new(folder: &Path, mut sessions: Vec<ProjectSession>) -> Option<Self> {
        sessions.sort_by_key(|entry| Reverse(entry.session.changed_at()));
        let changed_at = sessions.first()?.session.changed_at();

        Some(Self {
            folder: folder.to_path_buf(),
            changed_at,
            sessions,
        })
    }
}

/// A session of a project together with the profile it belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSession {
    profile: Profile,
    session: AgentSession,
}

impl ProjectSession {
    /// The profile the session belongs to.
    pub fn profile(&self) -> &Profile {
        &self.profile
    }

    /// The session.
    pub fn session(&self) -> &AgentSession {
        &self.session
    }
}

/// Groups the sessions of all profiles into projects, one for each folder.
///
/// The projects are sorted by their latest session, newest first, and the
/// sessions of each project are newest first too.
pub fn projects<'a>(
    sessions_by_profile: impl IntoIterator<Item = (&'a Profile, &'a [AgentSession])>,
) -> Vec<Project> {
    let mut by_folder: BTreeMap<&Path, Vec<ProjectSession>> = BTreeMap::new();
    for (profile, sessions) in sessions_by_profile {
        for session in sessions {
            let entry = ProjectSession {
                profile: profile.clone(),
                session: session.clone(),
            };
            by_folder.entry(session.folder()).or_default().push(entry);
        }
    }

    let mut projects: Vec<Project> = by_folder
        .into_iter()
        .filter_map(|(folder, sessions)| Project::new(folder, sessions))
        .collect();
    projects.sort_by_key(|project| Reverse(project.changed_at()));

    projects
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{profile, session};

    fn session_ids(project: &Project) -> Vec<&str> {
        project
            .sessions()
            .iter()
            .map(|entry| entry.session().id().as_str())
            .collect()
    }

    #[test]
    fn sessions_of_two_profiles_in_one_folder_become_one_project() {
        let claude = profile("claude");
        let codex = profile("codex");
        let claude_sessions = [session("c1", "/work/otty", 10)];
        let codex_sessions = [session("x1", "/work/otty", 20)];

        let projects = projects([
            (&claude, &claude_sessions[..]),
            (&codex, &codex_sessions[..]),
        ]);

        assert_eq!(projects.len(), 1);
        let project = &projects[0];
        assert_eq!(project.folder(), Path::new("/work/otty"));
        assert_eq!(session_ids(project), ["x1", "c1"]);
        assert_eq!(project.sessions()[0].profile(), &codex);
        assert_eq!(project.sessions()[1].profile(), &claude);
    }

    #[test]
    fn projects_are_sorted_by_their_latest_session() {
        let main = profile("main");
        let sessions = [
            session("old", "/work/old", 10),
            session("new-1", "/work/new", 5),
            session("middle", "/work/middle", 20),
            session("new-2", "/work/new", 30),
        ];

        let projects = projects([(&main, &sessions[..])]);

        let folders: Vec<_> =
            projects.iter().map(|project| project.folder()).collect();
        assert_eq!(
            folders,
            [
                Path::new("/work/new"),
                Path::new("/work/middle"),
                Path::new("/work/old"),
            ]
        );
        assert_eq!(session_ids(&projects[0]), ["new-2", "new-1"]);
        assert_eq!(projects[0].changed_at(), sessions[3].changed_at());
    }

    #[test]
    fn no_sessions_give_no_projects() {
        let main = profile("main");

        let projects = projects([(&main, &[][..])]);

        assert!(projects.is_empty());
    }
}

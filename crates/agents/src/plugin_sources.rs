//! The sessions of a profile as an agent part reads them, in one read of
//! the part; what the part answers it can not do becomes the error of that
//! read.

use std::path::PathBuf;
use std::sync::Arc;

use otty_plugin_host::agents::agent::{
    AgentCalls, HostError, ReadError, Session,
};
use otty_plugin_host::{CallError, CalledPart, PluginFailureKind};

use crate::error::{SourceError, Unavailable};
use crate::file_memory::FileMemory;
use crate::part_need::PartNeed;
use crate::plugin_places::place_items;
use crate::plugin_values::{part_profile, time_of};
use crate::profile::Profile;
use crate::read_failure::read_failure;
use crate::refused_login::RefusedLogin;
use crate::session::{AgentSession, SessionSource};
use crate::session_files::{
    FoundSession, PanelFiles, panel_files, read_session_files,
};
use crate::source_id::SourceId;

/// The name of the read of a part's sessions in reports.
const READ_SESSIONS: &str = "read-sessions";

/// The sessions of a profile, as the part reads them file by file, with
/// what each file told kept between refreshes.
pub(crate) struct PluginSessions {
    part: Arc<CalledPart>,
    memory: FileMemory<Vec<FoundSession>>,
}

impl PluginSessions {
    /// The sessions `part` reads.
    pub(crate) fn new(part: Arc<CalledPart>) -> Self {
        Self {
            part,
            memory: FileMemory::default(),
        }
    }

    /// The files of the session places of `profile`, and those tied to
    /// the sessions `wanted` picks, after a read of the sessions that
    /// brings what each file tells up to date: the files a statistics
    /// panel reads (issue 22).
    pub(crate) fn panel_files(
        &self,
        profile: &Profile,
        wanted: impl Fn(&AgentSession) -> bool,
    ) -> Result<PanelFiles, SourceError> {
        self.part.read(READ_SESSIONS, |batch| {
            let mut calls = batch.agent()?;

            self.read(&mut calls, profile)
        })??;

        Ok(panel_files(&self.memory, profile.config_dir(), wanted))
    }

    /// The sessions of `profile` in one read of the part: its session
    /// places, then what each file that is new or changed tells. A call
    /// that failed fails the read; what the part could not read fails the
    /// list only.
    fn read(
        &self,
        calls: &mut AgentCalls<'_, '_>,
        profile: &Profile,
    ) -> Result<Result<Vec<AgentSession>, SourceError>, CallError> {
        let asked = part_profile(profile);
        let places = calls.session_places(&asked)?;
        let items = place_items(&self.part, &places);
        let paths = items.keys().cloned().collect();

        let mut failed_call = None;
        let sessions = read_session_files(
            &self.memory,
            profile.config_dir(),
            paths,
            |path| {
                let Some(item) = items.get(path) else {
                    return Ok(Vec::new());
                };

                match calls.read_sessions(&asked, item) {
                    Ok(Ok(sessions)) => {
                        Ok(sessions.into_iter().map(found_session).collect())
                    },
                    Ok(Err(ReadError::Host(HostError::NotFound(_)))) => {
                        Ok(Vec::new())
                    },
                    Ok(Err(error)) => Err(read_error(
                        &self.part,
                        READ_SESSIONS,
                        &error,
                        SourceError::NotAvailable(Unavailable::NoSessionFiles),
                        Some(profile),
                    )),
                    Err(error) => {
                        let kind = PluginFailureKind::of_call(&error);
                        failed_call = Some(error);
                        Err(self.part.failure(READ_SESSIONS, kind).into())
                    },
                }
            },
        );

        match failed_call {
            Some(error) => Err(error),
            None => Ok(sessions),
        }
    }
}

impl SessionSource for Arc<PluginSessions> {
    fn id(&self) -> SourceId {
        SourceId::AgentPart
    }

    fn sessions(
        &self,
        profile: &Profile,
    ) -> Result<Vec<AgentSession>, SourceError> {
        self.part.read(READ_SESSIONS, |batch| {
            let mut calls = batch.agent()?;

            self.read(&mut calls, profile)
        })?
    }
}

/// The error of `call` of `part` that answered `error`: `not_supported`
/// when the part does not read this, a need of the part as the need the
/// user is asked, about `profile` for a call of an agent part, a refused
/// login as such, for the caller to ask the agent part that holds it, and
/// anything else, and a need OTTY can not show, as the plugin could not
/// read.
pub(crate) fn read_error(
    part: &CalledPart,
    call: &'static str,
    error: &ReadError,
    not_supported: SourceError,
    profile: Option<&Profile>,
) -> SourceError {
    let need = match error {
        ReadError::NotSupported => return not_supported,
        ReadError::LoginRefused => {
            let refused = RefusedLogin::new(part.named().clone());
            return SourceError::LoginRefused(Box::new(refused));
        },
        ReadError::NeedsUser(need) => PartNeed::of_part(part.named(), need),
        _ => None,
    };

    match (need, profile) {
        (Some(need), Some(profile)) => {
            let need = need.about_profile(profile.config_dir());
            SourceError::NeedsUser(Box::new(need))
        },
        (Some(need), None) => SourceError::NeedsUser(Box::new(need)),
        (None, _) => part.failure(call, read_failure(error)).into(),
    }
}

/// What the part's `session` tells about one session, as the host joins
/// it with what the other files tell.
fn found_session(session: Session) -> FoundSession {
    FoundSession::new(session.id, time_of(session.changed_ms))
        .with_folder(session.folder.map(PathBuf::from))
        .with_title(session.title)
}

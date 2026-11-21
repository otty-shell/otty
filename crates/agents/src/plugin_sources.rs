//! The sessions, limits and statistics of a profile as an agent part
//! reads them. Each is one read of the part; what the part answers it can
//! not do becomes the error of that read.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use otty_plugin_host::agents::agent::{
    AgentCalls, HostError, Login, LoginKind, ReadError, Session,
};
use otty_plugin_host::{CallBatch, CallError, CalledPart, PluginFailureKind};

use crate::agent::AgentId;
use crate::error::{SourceError, Unavailable};
use crate::file_memory::FileMemory;
use crate::limits::{LimitSource, Limits};
use crate::local_time::LocalNow;
use crate::part_need::PartNeed;
use crate::plugin_places::place_items;
use crate::plugin_values::{limits_of, models_of, part_profile, time_of};
use crate::profile::Profile;
use crate::read_failure::read_failure;
use crate::refused_login::RefusedLogin;
use crate::session::{AgentSession, SessionId, SessionSource};
use crate::session_files::{FoundSession, read_session_files};
use crate::source_id::SourceId;
use crate::stats::{ProjectStats, SessionStats, StatsSource};

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

/// The limits the agent itself shows for the profile's login, from
/// wherever the part reads them.
pub(crate) struct PluginLimits {
    part: Arc<CalledPart>,
    starts_programs: bool,
}

impl PluginLimits {
    /// The limits `part` reads; the read starts the programs and hidden
    /// terminals the part declares only when `starts_programs`.
    pub(crate) fn new(part: Arc<CalledPart>, starts_programs: bool) -> Self {
        Self {
            part,
            starts_programs,
        }
    }
}

impl LimitSource for PluginLimits {
    /// The limits of the profile's subscription login, else of its first
    /// login, in one read of the part; `None` when the part reads no
    /// limits.
    fn limits(&self, profile: &Profile) -> Result<Option<Limits>, SourceError> {
        let asked = part_profile(profile);

        let read = |batch: &mut CallBatch<'_>| {
            let mut calls = batch.agent()?;
            let logins = calls.logins(&asked)?;
            let Some(login) = limits_login(&logins) else {
                return Ok(Err(SourceError::NotAvailable(
                    Unavailable::NoLogin,
                )));
            };

            let limits = match calls.read_limits(&asked, &login.place)? {
                Ok(limits) => Ok(Some(limits_of(&limits))),
                Err(ReadError::NotSupported) => Ok(None),
                Err(error) => Err(read_error(
                    &self.part,
                    "read-limits",
                    &error,
                    no_limits(),
                    Some(profile),
                )),
            };
            Ok(limits)
        };

        if self.starts_programs {
            self.part.read("read-limits", read)?
        } else {
            self.part.read_starting_nothing("read-limits", read)?
        }
    }
}

/// The statistics of a session and of a project folder, as the part
/// reads them.
pub(crate) struct PluginStats {
    part: Arc<CalledPart>,
}

impl PluginStats {
    /// The statistics `part` reads.
    pub(crate) fn new(part: Arc<CalledPart>) -> Self {
        Self { part }
    }

    /// What a part that reads no statistics gives.
    fn no_statistics(&self) -> SourceError {
        let agent = AgentId::new(self.part.part().id());

        SourceError::NotAvailable(Unavailable::NoStatistics(agent))
    }
}

impl StatsSource for PluginStats {
    fn session_stats(
        &self,
        profile: &Profile,
        session: &SessionId,
        _now: &LocalNow,
    ) -> Result<SessionStats, SourceError> {
        let asked = part_profile(profile);
        let call = "read-session-stats";

        let stats = self
            .part
            .read(call, |batch| {
                batch.agent()?.read_session_stats(&asked, session.as_str())
            })?
            .map_err(|error| {
                let none = self.no_statistics();
                read_error(&self.part, call, &error, none, Some(profile))
            })?;

        Ok(SessionStats::new(
            stats.started_ms.map(time_of),
            stats.last_active_ms.map(time_of),
            stats.messages,
            models_of(&stats.models),
        ))
    }

    fn project_stats(
        &self,
        profile: &Profile,
        folder: &Path,
    ) -> Result<ProjectStats, SourceError> {
        let asked = part_profile(profile);
        let folder = folder.display().to_string();
        let call = "read-project-stats";

        let stats = self
            .part
            .read(call, |batch| {
                batch.agent()?.read_project_stats(&asked, &folder)
            })?
            .map_err(|error| {
                let none = self.no_statistics();
                read_error(&self.part, call, &error, none, Some(profile))
            })?;

        Ok(ProjectStats::new(
            stats.first_ms.map(time_of),
            stats.last_ms.map(time_of),
            models_of(&stats.models),
        ))
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

/// The login whose limits the part reads: a subscription, else the first.
fn limits_login(logins: &[Login]) -> Option<&Login> {
    logins
        .iter()
        .find(|login| matches!(login.kind, LoginKind::Subscription))
        .or_else(|| logins.first())
}

/// The error of "not supported" that [`read_error`] wants for the limits.
/// It is never given: [`PluginLimits`] answers "not supported" as `None`,
/// no limits, before it asks [`read_error`].
fn no_limits() -> SourceError {
    SourceError::NotAvailable(Unavailable::NoAccountSource)
}

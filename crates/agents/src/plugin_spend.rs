//! The tokens a profile spent, and the statistics of one session and of
//! one project, as an agent part reads their answers (spec
//! 2026-11-19-agent-plugins, "Host operations"; issue 22): the part names
//! its spend places, the host walks them, and the part reads only the
//! files that changed since they were last read. OTTY adds the answers
//! up, counts an answer with one identity once, and judges "format
//! changed" from what the part says it saw.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use otty_plugin_host::agents::agent::{
    AgentCalls, Answers, HostError, KeyPlace, Profile as PartProfile, ReadError,
};
use otty_plugin_host::{CallError, CalledPart, PluginFailureKind};

use crate::account::AccountIdentity;
use crate::dollars::Dollars;
use crate::error::SourceError;
use crate::file_memory::FileMemory;
use crate::local_time::LocalNow;
use crate::plugin_agent::account_of;
use crate::plugin_places::place_items;
use crate::plugin_sources::PluginSessions;
use crate::plugin_values::{
    is_same_place, part_profile, provider_of, time_of, tokens_of,
};
use crate::profile::Profile;
use crate::read_failure::read_failure;
use crate::record_format::{RecordCounts, RecordFormat};
use crate::session::SessionId;
use crate::session_files::PanelFiles;
use crate::spend::{Spend, SpendCounter};
use crate::spend_files::{
    CountedAnswer, FileSpend, count_spend, read_every_file,
};
use crate::stats::{ProjectStats, SessionStats, StatsSource};
use crate::stats_tally::{project_stats, session_stats};

/// The name of the read of a part's answers in reports.
const READ_ANSWERS: &str = "read-answers";

/// The spend counter and the statistics source of one agent part, with
/// what each file told kept between refreshes and shared by both.
pub(crate) struct PluginSpend {
    part: Arc<CalledPart>,
    sessions: Arc<PluginSessions>,
    tested_version: Option<String>,
    memory: FileMemory<FileSpend>,
    writes_cost: Arc<AtomicBool>,
}

impl PluginSpend {
    /// The counter of `part`, checked by its author up to the agent's
    /// version `tested_version`; its statistics read the files that
    /// `sessions` tie to their sessions. It sets `writes_cost` once the
    /// part gives the cost of an answer.
    pub(crate) fn new(
        part: Arc<CalledPart>,
        sessions: Arc<PluginSessions>,
        tested_version: Option<String>,
        writes_cost: Arc<AtomicBool>,
    ) -> Self {
        Self {
            part,
            sessions,
            tested_version,
            memory: FileMemory::default(),
            writes_cost,
        }
    }

    /// The count of `profile` at `now` in one read of the part: its spend
    /// places, then the answers of each file that changed. A call that
    /// failed fails the read; what the part could not read fails the
    /// count only.
    fn count(
        &self,
        calls: &mut AgentCalls<'_, '_>,
        profile: &Profile,
        now: &LocalNow,
    ) -> Result<Result<Spend, SourceError>, CallError> {
        let format = RecordFormat::of_plugin(
            self.part.named().clone(),
            self.tested_version.as_deref(),
        );

        self.walk(calls, profile, |paths, read_file| {
            count_spend(
                &self.memory,
                profile.config_dir(),
                paths,
                now,
                None,
                format,
                read_file,
            )
        })
    }

    /// What `made` makes of the answers of the files of the spend places
    /// of `profile` that a panel reads, `files`, over the whole history,
    /// in one read of the part, such as the statistics of a session.
    fn read_history<T>(
        &self,
        calls: &mut AgentCalls<'_, '_>,
        profile: &Profile,
        files: &PanelFiles,
        made: impl FnOnce(Vec<&FileSpend>) -> T,
    ) -> Result<Result<T, SourceError>, CallError> {
        self.walk(calls, profile, |paths, read_file| {
            read_every_file(
                &self.memory,
                profile.config_dir(),
                files.of(paths),
                read_file,
                made,
            )
        })
    }

    /// One walk of the spend places of `profile`: `walk` gets the paths of
    /// their files and reads with the part each file it needs. A call
    /// that failed fails the walk; what the part could not read fails
    /// what `walk` makes only.
    fn walk<T>(
        &self,
        calls: &mut AgentCalls<'_, '_>,
        profile: &Profile,
        walk: impl FnOnce(
            Vec<PathBuf>,
            &mut dyn FnMut(&Path) -> Result<FileSpend, SourceError>,
        ) -> Result<T, SourceError>,
    ) -> Result<Result<T, SourceError>, CallError> {
        let asked = part_profile(profile);
        let places = calls.spend_places(&asked)?;

        let items = place_items(&self.part, &places);
        let paths = items.keys().cloned().collect();

        let mut failed_call = None;
        let mut logins = None;
        let made = walk(paths, &mut |path| {
            let Some(item) = items.get(path) else {
                return Ok(FileSpend::default());
            };

            match calls.read_answers(&asked, item) {
                Ok(Ok(answers)) => {
                    let names_a_login = answers
                        .answers
                        .iter()
                        .any(|answer| answer.login.is_some());
                    if names_a_login && logins.is_none() {
                        logins =
                            Some(self.login_accounts(calls, &asked, profile));
                    }
                    let logins = logins.as_deref().unwrap_or_default();

                    Ok(self.file_spend(answers, logins))
                },
                Ok(Err(ReadError::Host(HostError::NotFound(_)))) => {
                    Ok(FileSpend::default())
                },
                Ok(Err(error)) => Err(self.could_not_read(&error)),
                Err(error) => {
                    let kind = PluginFailureKind::of_call(&error);
                    failed_call = Some(error);
                    Err(self.part.failure(READ_ANSWERS, kind).into())
                },
            }
        });

        match failed_call {
            Some(error) => Err(error),
            None => Ok(made),
        }
    }

    /// The account of each login the part finds in `profile`, by the
    /// place of its key: what an answer that names its login was paid by.
    fn login_accounts(
        &self,
        calls: &mut AgentCalls<'_, '_>,
        asked: &PartProfile,
        profile: &Profile,
    ) -> Vec<(KeyPlace, AccountIdentity)> {
        let logins = calls.logins(asked).unwrap_or_default();

        logins
            .iter()
            .filter_map(|login| {
                let account = account_of(&self.part, login, profile)?;
                Some((login.place.clone(), account.identity().clone()))
            })
            .collect()
    }

    /// What one file told, from the part's `answers`: each answer with
    /// its own project, provider and account, and what the part saw.
    fn file_spend(
        &self,
        answers: Answers,
        logins: &[(KeyPlace, AccountIdentity)],
    ) -> FileSpend {
        let records = &answers.records;
        let records = RecordCounts::of_plugin(
            records.seen,
            records.understood,
            records.version.as_deref(),
            records.missing.clone(),
            records.fields.clone(),
        );

        let counted = answers
            .answers
            .into_iter()
            .map(|answer| {
                if answer.cost_microdollars.is_some() {
                    self.writes_cost.store(true, Ordering::Relaxed);
                }
                let account = answer.login.as_ref().and_then(|login| {
                    logins
                        .iter()
                        .find(|(place, _)| is_same_place(place, login))
                        .map(|(_, identity)| identity.clone())
                });

                let counted = CountedAnswer::new(
                    time_of(answer.at_ms),
                    answer.model.as_deref(),
                    tokens_of(&answer.tokens),
                )
                .with_cost(
                    answer.cost_microdollars.map(Dollars::from_micro_dollars),
                )
                .with_session(answer.session)
                .with_project(answer.project.map(PathBuf::from))
                .with_provider(answer.provider.as_ref().map(provider_of))
                .with_account(account);

                match &answer.identity {
                    Some(identity) => counted.with_identity(identity),
                    None => counted,
                }
            })
            .collect();

        FileSpend::new(None, counted).with_records(records)
    }

    /// The error of a read the part answered it could not do.
    fn could_not_read(&self, error: &ReadError) -> SourceError {
        let kind = read_failure(error);

        self.part.failure(READ_ANSWERS, kind).into()
    }
}

impl SpendCounter for Arc<PluginSpend> {
    fn spend(
        &self,
        profile: &Profile,
        now: &LocalNow,
    ) -> Result<Spend, SourceError> {
        self.part.read(READ_ANSWERS, |batch| {
            let mut calls = batch.agent()?;

            self.count(&mut calls, profile, now)
        })?
    }
}

impl StatsSource for Arc<PluginSpend> {
    fn session_stats(
        &self,
        profile: &Profile,
        session: &SessionId,
        now: &LocalNow,
    ) -> Result<SessionStats, SourceError> {
        let files = self
            .sessions
            .panel_files(profile, |listed| listed.id() == session)?;

        self.part.read(READ_ANSWERS, |batch| {
            let mut calls = batch.agent()?;

            self.read_history(&mut calls, profile, &files, |files| {
                session_stats(&files, session, now)
            })
        })?
    }

    fn project_stats(
        &self,
        profile: &Profile,
        folder: &Path,
    ) -> Result<ProjectStats, SourceError> {
        let files = self
            .sessions
            .panel_files(profile, |listed| listed.folder() == folder)?;

        self.part.read(READ_ANSWERS, |batch| {
            let mut calls = batch.agent()?;

            self.read_history(&mut calls, profile, &files, |files| {
                project_stats(&files, folder)
            })
        })?
    }
}

#[cfg(test)]
mod tests {
    use otty_plugin_host::PluginFailure;

    use super::*;
    use crate::account::{Account, AccountKind, AccountLogin, FoundIn};
    use crate::agent::Agent;
    use crate::plugin_agent::PluginAgent;
    use crate::plugin_provider::PluginProvider;
    use crate::provider::ProviderRef;
    use crate::spend::ModelTokens;
    use crate::testing::{NOW, TemplateHome, answer_line, meta_line, utc_now};
    use crate::token_count::TokenCount;

    const AGENT: &str = "example-agent";
    const HOUR: u64 = 3600;

    fn agent(home: &TemplateHome) -> PluginAgent {
        PluginAgent::new(home.plugin(), AGENT).expect("the agent part")
    }

    fn spend(
        agent: &PluginAgent,
        home: &TemplateHome,
    ) -> Result<Spend, SourceError> {
        let counter = agent.spend_counter().expect("a spend counter");

        counter.spend(&home.profile(), &utc_now())
    }

    fn tokens(input: u64, output: u64) -> TokenCount {
        TokenCount::new(input, 0, output)
    }

    /// Issue 07: the template plugin as an agent gives the tokens of its
    /// made-up session files, by project and model, under its provider.
    #[test]
    fn the_template_gives_the_tokens_of_its_session_files() {
        let home = TemplateHome::new();
        home.write_session(
            ".example-agent",
            "s1.jsonl",
            &[
                meta_line("/home/dev/shop"),
                answer_line("a1", NOW - HOUR, 450),
                answer_line("a2", NOW - HOUR, 200),
            ],
            NOW,
        );
        home.write_session(
            ".example-agent",
            "s2.jsonl",
            &[meta_line("/home/dev/blog"), answer_line("b1", NOW, 900)],
            NOW,
        );

        let spend = spend(&agent(&home), &home).expect("the spend");

        assert_eq!(spend.today(), tokens(300, 1550));
        let projects: Vec<_> = spend
            .projects()
            .iter()
            .map(|project| (project.folder().to_path_buf(), project.tokens()))
            .collect();
        assert_eq!(
            projects,
            [
                (Path::new("/home/dev/blog").to_path_buf(), tokens(100, 900)),
                (Path::new("/home/dev/shop").to_path_buf(), tokens(200, 650)),
            ]
        );
        assert_eq!(
            spend.this_month_by_model(),
            [
                ModelTokens::new(Some("example-large"), tokens(300, 1550), 3)
                    .with_provider(Some(ProviderRef::of_id("example-cloud")))
            ]
        );
    }

    /// Spec "Host operations": a file whose size and change time did not
    /// change is not read again.
    #[test]
    fn a_file_that_did_not_change_is_not_read_again() {
        let home = TemplateHome::new();
        let lines = |output| [meta_line("/p"), answer_line("a1", NOW, output)];
        home.write_session(".example-agent", "s1.jsonl", &lines(100), NOW);
        let agent = agent(&home);

        let first = spend(&agent, &home).expect("the spend");
        home.write_session(".example-agent", "s1.jsonl", &lines(900), NOW);
        let same_stamp = spend(&agent, &home).expect("the spend");
        home.write_session(".example-agent", "s1.jsonl", &lines(900), NOW - 1);
        let changed = spend(&agent, &home).expect("the spend");

        assert_eq!(first.today(), tokens(100, 100));
        assert_eq!(same_stamp.today(), tokens(100, 100));
        assert_eq!(changed.today(), tokens(100, 900));
    }

    /// Spec Decision 6: an answer with one identity in two files counts
    /// once, with its most output.
    #[test]
    fn the_same_answer_in_two_files_counts_once() {
        let home = TemplateHome::new();
        home.write_session(
            ".example-agent",
            "s1.jsonl",
            &[meta_line("/p"), answer_line("a1", NOW, 100)],
            NOW,
        );
        home.write_session(
            ".example-agent",
            "s2.jsonl",
            &[meta_line("/p"), answer_line("a1", NOW, 300)],
            NOW,
        );

        let spend = spend(&agent(&home), &home).expect("the spend");

        assert_eq!(spend.today(), tokens(100, 300));
    }

    /// Issue 22: the panels of a session and of a project are the sums of
    /// the answers that name the session or the project, an answer written
    /// twice counts once, and the project agrees with the totals; the
    /// project reaches past the counted days.
    #[test]
    fn the_panels_sum_the_answers_of_a_session_and_of_a_project() {
        let home = TemplateHome::new();
        let long_ago = NOW - 400 * 24 * HOUR;
        home.write_session(
            ".example-agent",
            "s1.jsonl",
            &[
                meta_line("/home/dev/shop"),
                answer_line("a1", NOW - HOUR, 450),
                answer_line("a2", NOW - HOUR, 200),
            ],
            NOW,
        );
        home.write_session(
            ".example-agent",
            "s2.jsonl",
            &[
                meta_line("/home/dev/shop"),
                answer_line("a1", NOW - HOUR, 450),
                answer_line("b1", NOW, 900),
            ],
            NOW,
        );
        home.write_session(
            ".example-agent",
            "s3.jsonl",
            &[meta_line("/home/dev/blog"), answer_line("c1", long_ago, 5)],
            long_ago,
        );
        let agent = agent(&home);
        let source = agent.stats_source().expect("a statistics source");

        let session = source
            .session_stats(&home.profile(), &SessionId::new("s1"), &utc_now())
            .expect("the session");
        let shop = source
            .project_stats(&home.profile(), Path::new("/home/dev/shop"))
            .expect("the project");
        let blog = source
            .project_stats(&home.profile(), Path::new("/home/dev/blog"))
            .expect("the old project");
        let totals = spend(&agent, &home).expect("the spend");

        assert_eq!(session.tokens(), tokens(200, 650));
        assert_eq!(session.answers(), 2);
        assert_eq!(session.started_at(), Some(crate::testing::at(NOW - HOUR)));
        assert_eq!(session.spend().today(), tokens(200, 650));
        assert_eq!(shop.tokens(), tokens(300, 1550));
        assert_eq!(totals.projects()[0].tokens(), shop.tokens());
        assert_eq!(shop.models(), totals.this_month_by_model());
        assert_eq!(blog.tokens(), tokens(100, 5));
        assert_eq!(blog.first_at(), Some(crate::testing::at(long_ago)));
    }

    /// Spec "What the UI shows": newest files the part understands less
    /// than half of are a changed format, named with the plugin.
    #[test]
    fn records_the_part_does_not_understand_change_the_format() {
        let home = TemplateHome::new();
        let mut lines = vec![meta_line("/p")];
        for number in 0..6 {
            lines.push(format!(
                r#"{{"type":"answer","id":"x{number}","at_ms":{},"usage":{{"in":1}}}}"#,
                NOW * 1000
            ));
        }
        home.write_session(".example-agent", "s1.jsonl", &lines, NOW);

        let error = spend(&agent(&home), &home).expect_err("a changed format");

        let SourceError::FormatChanged(change) = error else {
            panic!("a changed format: {error:?}");
        };
        assert_eq!((change.seen(), change.understood()), (6, 0));
        assert_eq!(change.version(), Some("1.4.2"));
        assert_eq!(change.tested_version(), "1.4.2");
        assert_eq!(change.plugin().map(|part| part.plugin()), Some("example"));
        let report = change.report("Example Agent", "~/.example-agent");
        assert!(report.contains("Plugin: example 0.1.0"), "{report}");
        assert!(report.contains("Part: example-agent"), "{report}");
    }

    /// Spec Decision 8: a trap fails that read only; three in a row pause
    /// the part, which is then not called, while the plugin's provider
    /// part keeps reading.
    #[test]
    fn a_trapping_part_is_paused_after_three_and_the_other_part_reads() {
        let home = TemplateHome::new();
        let zeros = vec!["0"; 4 * 1024 * 1024].join(",");
        let huge = format!(r#"{{"type":"answer","id":"h","pad":[{zeros}]}}"#);
        home.write_session(".example-agent", "s1.jsonl", &[huge], NOW);
        let plugin = home.plugin();
        let agent =
            PluginAgent::new(Arc::clone(&plugin), AGENT).expect("the agent");
        let provider = PluginProvider::new(plugin, "example-cloud")
            .expect("the provider part");

        let failures: Vec<_> = (0..4)
            .map(|_| match spend(&agent, &home) {
                Err(SourceError::PluginFailed(failure)) => failure.kind(),
                other => panic!("a failed read: {other:?}"),
            })
            .collect();
        let other_part =
            provider.read(&cloud_account(&home), &missing_key(), &[]);

        assert_eq!(
            failures,
            [
                PluginFailureKind::Stopped,
                PluginFailureKind::Stopped,
                PluginFailureKind::Stopped,
                PluginFailureKind::Paused,
            ]
        );
        assert!(agent.health().is_paused());
        assert!(!provider.health().is_paused());
        let Err(SourceError::PluginFailed(failure)) = other_part else {
            panic!("the provider part read and found no key: {other_part:?}");
        };
        assert_eq!(
            *failure,
            PluginFailure::new(
                failure.part().clone(),
                "numbers",
                PluginFailureKind::CouldNotRead("host error: not found"),
            )
        );
    }

    fn cloud_account(home: &TemplateHome) -> Account {
        Account::new(
            ProviderRef::of_id("example-cloud"),
            AccountIdentity::Email("dev@example.com".into()),
            AccountLogin::Email("dev@example.com".into()),
            AccountKind::Money,
            FoundIn::ProfileFolder(home.folder()),
        )
    }

    /// A login whose key file is not there: the host finds no key, so
    /// nothing is sent.
    fn missing_key() -> otty_plugin_host::agents::agent::Login {
        use otty_plugin_host::agents::agent::{JsonField, Login, LoginKind};

        Login {
            kind: LoginKind::ApiKey,
            for_whom: otty_plugin_host::agents::agent::ProviderRef::Id(
                "example-cloud".to_string(),
            ),
            place: KeyPlace::JsonField(JsonField {
                file: "/no/such/home/.example-agent/auth.json".to_string(),
                pointer: "/api_key".to_string(),
            }),
            identity: crate::testing::no_identity(),
            plan: None,
        }
    }
}

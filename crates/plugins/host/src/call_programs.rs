//! The programs of the running call: the programs a part runs once and
//! the ones it talks to line by line. Each is checked against the
//! permissions of the called part, kept to its time and output limits,
//! and stopped when the call that started it ends (spec "Host
//! operations").

use std::sync::Arc;
use std::time::{Duration, Instant};

use otty_plugin_manifest::Permissions;

use crate::operations::v0_1::otty::host::types::HostError;
use crate::program_paths::ProgramPaths;
use crate::programs::{
    LineOutput, ProgramCommand, ProgramError, RunOutput, RunningProgram,
    StartProgram,
};

/// The longest a program run once may take.
pub(crate) const RUN_TIME_CAP: Duration = Duration::from_secs(10);
/// The most bytes a program run once may write, both outputs together.
pub(crate) const RUN_OUTPUT_CAP: usize = 1024 * 1024;
/// The longest a talk may last, from its start.
pub(crate) const TALK_TIME: Duration = Duration::from_secs(60);
/// The most programs one call may talk to at once, and the most hidden
/// terminals it may keep at once, so a plugin can not start programs
/// without end.
pub(crate) const RUNNING_CAP: usize = 4;

/// The host side of a `talk` resource: which talk of which call of the
/// batch. A handle kept past its call finds nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TalkHandle {
    call: u64,
    slot: usize,
}

/// The programs of the running call of one batch.
pub(crate) struct CallPrograms {
    starter: Option<Arc<dyn StartProgram>>,
    permissions: Arc<Permissions>,
    paths: Arc<ProgramPaths>,
    call: u64,
    talks: Vec<Option<Talk>>,
    has_started: bool,
}

impl CallPrograms {
    /// Whether the running call has run or started a program.
    pub(crate) fn has_started(&self) -> bool {
        self.has_started
    }

    /// The programs of the calls of a part with `permissions`, started by
    /// `starter`, those found at a program place by their full path in
    /// `paths`; with no starter, every program is refused.
    pub(crate) fn new(
        starter: Option<Arc<dyn StartProgram>>,
        permissions: Arc<Permissions>,
        paths: Arc<ProgramPaths>,
    ) -> Self {
        Self {
            starter,
            permissions,
            paths,
            call: 0,
            talks: Vec::new(),
            has_started: false,
        }
    }

    /// A call begins: nothing has started in it yet.
    pub(crate) fn start_call(&mut self) {
        self.has_started = false;
    }

    /// The call ended: every program it talks to is stopped, and the
    /// handles it gave find nothing any more.
    pub(crate) fn end_call(&mut self) {
        self.stop_all();
        self.call += 1;
    }

    /// Runs `program` with `args` and the variables `env` once, for at
    /// most `timeout` and 10 s, and gives what it wrote.
    pub(crate) fn run(
        &mut self,
        program: &str,
        args: &[String],
        env: &[(String, String)],
        timeout: Duration,
    ) -> Result<RunOutput, HostError> {
        let name = self.paths.name_of(program);
        let may_run = self.permissions.may_run(&name, args);
        let command = checked_command(
            &self.permissions,
            may_run,
            program,
            &self.paths.to_start(&name),
            args,
            env,
        )?;
        let starter = self.starter()?;

        self.has_started = true;
        let until = Instant::now() + timeout.min(RUN_TIME_CAP);
        let output = starter
            .run(&command, until, RUN_OUTPUT_CAP)
            .map_err(|error| host_error(program, &error))?;

        if output.written_bytes() > RUN_OUTPUT_CAP {
            return Err(host_error(
                program,
                &ProgramError::TooMuchOutput(RUN_OUTPUT_CAP),
            ));
        }

        Ok(output)
    }

    /// Starts `program` with `args` and the variables `env` to talk to it
    /// line by line, for at most 60 s.
    pub(crate) fn start_talk(
        &mut self,
        program: &str,
        args: &[String],
        env: &[(String, String)],
    ) -> Result<TalkHandle, HostError> {
        let name = self.paths.name_of(program);
        let may_run = self.permissions.may_run(&name, args);
        let command = checked_command(
            &self.permissions,
            may_run,
            program,
            &self.paths.to_start(&name),
            args,
            env,
        )?;
        if self.talks.iter().flatten().count() >= RUNNING_CAP {
            return Err(HostError::Failed(format!(
                "a call may talk to at most {RUNNING_CAP} programs at once"
            )));
        }
        let starter = self.starter()?;

        self.has_started = true;
        let running = starter
            .start(&command)
            .map_err(|error| host_error(program, &error))?;

        let talk = Talk {
            name: program.to_string(),
            program: running,
            deadline: Instant::now() + TALK_TIME,
        };
        let slot = free_slot(&mut self.talks);
        self.talks[slot] = Some(talk);

        Ok(TalkHandle {
            call: self.call,
            slot,
        })
    }

    /// Writes `line` to the program of `talk`.
    pub(crate) fn send_line(
        &mut self,
        talk: TalkHandle,
        line: &str,
    ) -> Result<(), HostError> {
        let now = Instant::now();
        self.expire_past(talk, now)?;

        let talk = self.talk(talk)?;
        talk.program
            .send_line(line)
            .map_err(|error| host_error(&talk.name, &error))
    }

    /// The next line the program of `talk` writes within `timeout` and
    /// the rest of the talk's 60 s; `None` when it ended.
    pub(crate) fn next_line(
        &mut self,
        talk: TalkHandle,
        timeout: Duration,
    ) -> Result<Option<String>, HostError> {
        let now = Instant::now();
        self.expire_past(talk, now)?;

        let talk = self.talk(talk)?;
        let until = (now + timeout).min(talk.deadline);
        match talk.program.next_line(until) {
            LineOutput::Line(line) => Ok(Some(line)),
            LineOutput::Ended => Ok(None),
            LineOutput::TimedOut => Err(HostError::TimedOut(format!(
                "{} wrote no line in time",
                talk.name
            ))),
        }
    }

    /// The plugin dropped `talk`: its program is stopped.
    pub(crate) fn drop_talk(&mut self, talk: TalkHandle) {
        if talk.call != self.call {
            return;
        }

        if let Some(mut talk) =
            self.talks.get_mut(talk.slot).and_then(Option::take)
        {
            talk.program.stop();
        }
    }

    fn starter(&self) -> Result<Arc<dyn StartProgram>, HostError> {
        self.starter.clone().ok_or_else(|| {
            HostError::Failed(
                "this OTTY starts no programs for plugins".to_string(),
            )
        })
    }

    /// The running talk of `handle`, if its call still runs.
    fn talk(&mut self, handle: TalkHandle) -> Result<&mut Talk, HostError> {
        let talk = (handle.call == self.call)
            .then(|| self.talks.get_mut(handle.slot))
            .flatten()
            .and_then(Option::as_mut);

        talk.ok_or_else(|| {
            HostError::Failed(
                "the talk ended with the call that started it".to_string(),
            )
        })
    }

    /// Stops the talk of `handle` once its 60 s have passed at `now`.
    fn expire_past(
        &mut self,
        handle: TalkHandle,
        now: Instant,
    ) -> Result<(), HostError> {
        let talk = self.talk(handle)?;
        if now < talk.deadline {
            return Ok(());
        }

        let name = talk.name.clone();
        self.drop_talk(handle);

        Err(HostError::TimedOut(format!(
            "the talk with {name} ran past {} s and was stopped",
            TALK_TIME.as_secs()
        )))
    }

    fn stop_all(&mut self) {
        for mut talk in self.talks.drain(..).flatten() {
            talk.program.stop();
        }
    }
}

impl Drop for CallPrograms {
    fn drop(&mut self) {
        self.stop_all();
    }
}

/// A program the running call talks to.
struct Talk {
    name: String,
    program: Box<dyn RunningProgram>,
    deadline: Instant,
}

/// The command that starts the program the part asked for as `asked`
/// by `started`, with `args` and the variables `env`, once the called part
/// may start it (`allowed`) and lists every variable in `env`. A variable
/// it does not list is refused, so it never reaches the program.
pub(crate) fn checked_command(
    permissions: &Permissions,
    allowed: bool,
    asked: &str,
    started: &str,
    args: &[String],
    env: &[(String, String)],
) -> Result<ProgramCommand, HostError> {
    if !allowed {
        return Err(HostError::NotAllowed(asked.to_string()));
    }
    if let Some((name, _)) =
        env.iter().find(|(name, _)| !permissions.may_read_var(name))
    {
        return Err(HostError::NotAllowed(format!("variable {name}")));
    }

    Ok(ProgramCommand::new(started, args, env))
}

/// `error` of `program` in the words of a host error.
pub(crate) fn host_error(program: &str, error: &ProgramError) -> HostError {
    let text = format!("{program}: {error}");

    match error {
        ProgramError::TimedOut => HostError::TimedOut(text),
        ProgramError::TooMuchOutput(_) => HostError::TooBig(text),
        _ => HostError::Failed(text),
    }
}

/// The first free slot of `slots`, a new one at the end when none is.
pub(crate) fn free_slot<T>(slots: &mut Vec<Option<T>>) -> usize {
    if let Some(free) = slots.iter().position(Option::is_none) {
        return free;
    }

    slots.push(None);
    slots.len() - 1
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Mutex;

    use mockall::predicate::always;
    use otty_plugin_manifest::Environment;

    use super::*;
    use crate::programs::{MockRunningProgram, MockStartProgram};
    use crate::testing::{assert_same, programs_permissions, strings};

    fn env() -> Environment {
        Environment::new("/home/test-user".into())
    }

    fn programs(starter: MockStartProgram) -> CallPrograms {
        CallPrograms::new(
            Some(Arc::new(starter)),
            programs_permissions("test-agent", &env()),
            Arc::new(ProgramPaths::default()),
        )
    }

    fn vars(vars: &[(&str, &str)]) -> Vec<(String, String)> {
        vars.iter()
            .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
            .collect()
    }

    /// A starter whose talks write `lines` and are stopped once.
    fn talking(lines: &[&str]) -> MockStartProgram {
        let lines = Mutex::new(strings(lines).into_iter());
        let mut program = MockRunningProgram::new();
        program.expect_send_line().returning(|_| Ok(()));
        program.expect_next_line().returning(move |_| {
            let mut lines = lines.lock().expect("lines");
            lines.next().map_or(LineOutput::Ended, LineOutput::Line)
        });
        program.expect_stop().times(1).return_const(());

        let mut starter = MockStartProgram::new();
        starter
            .expect_start()
            .times(1)
            .return_once(move |_| Ok(Box::new(program)));
        starter
    }

    #[test]
    fn a_declared_program_runs_with_its_listed_variables() {
        let mut starter = MockStartProgram::new();
        starter
            .expect_run()
            .withf(|command, _, max_output| {
                command.program() == "stand-in"
                    && command.args() == ["--version"]
                    && command.env()
                        == vars(&[("STAND_IN_HOME", "/data/stand-in")])
                    && *max_output == RUN_OUTPUT_CAP
            })
            .times(1)
            .returning(|_, _, _| {
                Ok(RunOutput::new(
                    Some(0),
                    b"stand-in 1.2.3\n".to_vec(),
                    vec![],
                ))
            });
        let mut programs = programs(starter);

        let output = programs.run(
            "stand-in",
            &strings(&["--version"]),
            &vars(&[("STAND_IN_HOME", "/data/stand-in")]),
            Duration::from_secs(5),
        );

        assert_same(
            output,
            Ok(RunOutput::new(
                Some(0),
                b"stand-in 1.2.3\n".to_vec(),
                vec![],
            )),
        );
        assert!(programs.has_started());
    }

    /// Issue 32: the login shell does not find `stand-in` by its name,
    /// but OTTY found it at a program place of the part. The part asks
    /// for it by its name or by that full path, the permission of the
    /// name applies, and it runs by the full path; any other path is
    /// refused.
    #[test]
    fn a_program_found_at_a_place_runs_by_its_full_path() {
        const FOUND: &str = "/home/test-user/.stand-in/bin/stand-in";
        let paths = Arc::new(ProgramPaths::default());
        paths.set("stand-in", Some(PathBuf::from(FOUND)));
        let mut starter = MockStartProgram::new();
        starter
            .expect_run()
            .withf(|command, _, _| {
                command.program() == FOUND && command.args() == ["--version"]
            })
            .times(2)
            .returning(|_, _, _| Ok(RunOutput::default()));
        starter
            .expect_start()
            .withf(|command| command.program() == FOUND)
            .times(1)
            .return_once(|_| {
                let mut program = MockRunningProgram::new();
                program.expect_stop().return_const(());
                Ok(Box::new(program))
            });
        let mut programs = CallPrograms::new(
            Some(Arc::new(starter)),
            programs_permissions("test-agent", &env()),
            paths,
        );
        let version = strings(&["--version"]);

        let by_name = programs.run("stand-in", &version, &[], RUN_TIME_CAP);
        let by_path = programs.run(FOUND, &version, &[], RUN_TIME_CAP);
        let other = programs.run("/tmp/stand-in", &version, &[], RUN_TIME_CAP);
        let serve = strings(&["serve", "--profile", "work"]);
        let talk = programs.start_talk("stand-in", &serve, &[]);

        assert!(by_name.is_ok(), "{by_name:?}");
        assert!(by_path.is_ok(), "{by_path:?}");
        assert_same(
            other,
            Err(HostError::NotAllowed("/tmp/stand-in".to_string())),
        );
        assert!(talk.is_ok(), "{talk:?}");
    }

    #[test]
    fn an_undeclared_program_or_argument_is_refused_and_nothing_starts() {
        let mut starter = MockStartProgram::new();
        starter.expect_run().never();
        starter.expect_start().never();
        let mut programs = programs(starter);

        let other_program =
            programs.run("sh", &strings(&["--version"]), &[], Duration::ZERO);
        let other_argument = programs.run(
            "stand-in",
            &strings(&["--version", "--yolo"]),
            &[],
            Duration::ZERO,
        );
        let talk = programs.start_talk("stand-in", &strings(&["exec"]), &[]);

        assert_same(
            other_program,
            Err(HostError::NotAllowed("sh".to_string())),
        );
        assert_same(
            other_argument,
            Err(HostError::NotAllowed("stand-in".to_string())),
        );
        assert_same(talk, Err(HostError::NotAllowed("stand-in".to_string())));
        assert!(!programs.has_started());
    }

    #[test]
    fn a_program_only_another_part_declares_is_refused() {
        let mut starter = MockStartProgram::new();
        starter.expect_run().never();
        let mut as_test_agent = programs(starter);
        let mut other_starter = MockStartProgram::new();
        other_starter
            .expect_run()
            .times(1)
            .returning(|_, _, _| Ok(RunOutput::default()));
        let mut as_other_agent = CallPrograms::new(
            Some(Arc::new(other_starter)),
            programs_permissions("other-agent", &env()),
            Arc::new(ProgramPaths::default()),
        );

        let refused = as_test_agent.run(
            "other",
            &strings(&["--version"]),
            &[],
            RUN_TIME_CAP,
        );
        let allowed = as_other_agent.run(
            "other",
            &strings(&["--version"]),
            &[],
            RUN_TIME_CAP,
        );

        assert_same(refused, Err(HostError::NotAllowed("other".to_string())));
        assert_same(allowed, Ok(RunOutput::default()));
    }

    #[test]
    fn a_variable_the_part_does_not_list_is_not_passed() {
        let mut starter = MockStartProgram::new();
        starter.expect_run().never();
        starter.expect_start().never();
        let mut programs = programs(starter);
        let unlisted = vars(&[("STAND_IN_HOME", "/data"), ("PATH", "/evil")]);
        let key = vars(&[("STAND_IN_KEY", "made-up")]);

        let run = programs.run(
            "stand-in",
            &strings(&["--version"]),
            &unlisted,
            RUN_TIME_CAP,
        );
        let talk = programs.start_talk(
            "stand-in",
            &strings(&["serve", "--profile", "work"]),
            &key,
        );

        assert_same(
            run,
            Err(HostError::NotAllowed("variable PATH".to_string())),
        );
        assert_same(
            talk,
            Err(HostError::NotAllowed("variable STAND_IN_KEY".to_string())),
        );
    }

    #[test]
    fn a_run_is_given_its_timeout_and_never_more_than_ten_seconds() {
        let untils = Arc::new(Mutex::new(Vec::new()));
        let seen = Arc::clone(&untils);
        let mut starter = MockStartProgram::new();
        starter.expect_run().times(2).returning(move |_, until, _| {
            seen.lock().expect("untils").push(until);
            Err(ProgramError::TimedOut)
        });
        let mut programs = programs(starter);
        let started = Instant::now();

        let short = programs.run(
            "stand-in",
            &strings(&["--version"]),
            &[],
            Duration::from_millis(300),
        );
        let long = programs.run(
            "stand-in",
            &strings(&["--version"]),
            &[],
            Duration::from_secs(600),
        );

        let untils = untils.lock().expect("untils");
        assert!(untils[0] >= started + Duration::from_millis(300));
        assert!(untils[0] < started + Duration::from_secs(1));
        assert!(untils[1] <= Instant::now() + RUN_TIME_CAP);
        assert!(untils[1] > started + Duration::from_secs(9));
        assert_same(
            short,
            Err(HostError::TimedOut(
                "stand-in: it ran past its time limit and was stopped"
                    .to_string(),
            )),
        );
        assert!(matches!(long, Err(HostError::TimedOut(_))));
    }

    #[test]
    fn more_output_than_the_cap_is_refused() {
        let mut starter = MockStartProgram::new();
        starter.expect_run().times(1).returning(|_, _, _| {
            Ok(RunOutput::new(
                Some(0),
                vec![b'a'; RUN_OUTPUT_CAP],
                b"and more".to_vec(),
            ))
        });
        let mut programs = programs(starter);

        let output = programs.run(
            "stand-in",
            &strings(&["--version"]),
            &[],
            RUN_TIME_CAP,
        );

        assert!(matches!(output, Err(HostError::TooBig(_))), "{output:?}");
    }

    #[test]
    fn no_starter_starts_nothing() {
        let mut programs = CallPrograms::new(
            None,
            programs_permissions("test-agent", &env()),
            Arc::new(ProgramPaths::default()),
        );

        let output = programs.run(
            "stand-in",
            &strings(&["--version"]),
            &[],
            RUN_TIME_CAP,
        );

        assert_same(
            output,
            Err(HostError::Failed(
                "this OTTY starts no programs for plugins".to_string(),
            )),
        );
    }

    #[test]
    fn a_talk_sends_lines_and_reads_them_until_the_program_ends() {
        let mut programs = programs(talking(&["{\"id\":1}"]));

        let talk = programs
            .start_talk(
                "stand-in",
                &strings(&["serve", "--profile", "work"]),
                &[],
            )
            .expect("a talk");
        let sent = programs.send_line(talk, "{\"method\":\"hello\"}");
        let first = programs.next_line(talk, Duration::from_secs(1));
        let second = programs.next_line(talk, Duration::from_secs(1));
        programs.end_call();

        assert_same(sent, Ok(()));
        assert_same(first, Ok(Some("{\"id\":1}".to_string())));
        assert_same(second, Ok(None));
    }

    #[test]
    fn a_talk_ends_with_the_call() {
        let mut programs = programs(talking(&["late"]));
        programs.start_call();
        let talk = programs
            .start_talk(
                "stand-in",
                &strings(&["serve", "--profile", "work"]),
                &[],
            )
            .expect("a talk");
        assert!(programs.has_started());

        programs.end_call();
        programs.start_call();

        assert!(!programs.has_started());
        assert_same(
            programs.next_line(talk, Duration::from_secs(1)),
            Err(HostError::Failed(
                "the talk ended with the call that started it".to_string(),
            )),
        );
    }

    #[test]
    fn a_program_that_never_answers_is_stopped_at_its_timeout() {
        let untils = Arc::new(Mutex::new(Vec::new()));
        let seen = Arc::clone(&untils);
        let mut program = MockRunningProgram::new();
        program.expect_next_line().returning(move |until| {
            seen.lock().expect("untils").push(until);
            LineOutput::TimedOut
        });
        program.expect_stop().times(1).return_const(());
        let mut starter = MockStartProgram::new();
        starter
            .expect_start()
            .return_once(move |_| Ok(Box::new(program)));
        let mut programs = programs(starter);
        let started = Instant::now();

        let talk = programs
            .start_talk(
                "stand-in",
                &strings(&["serve", "--profile", "work"]),
                &[],
            )
            .expect("a talk");
        let short = programs.next_line(talk, Duration::from_millis(200));
        let long = programs.next_line(talk, Duration::from_secs(600));
        programs.end_call();

        let untils = untils.lock().expect("untils");
        assert!(untils[0] < started + Duration::from_secs(1));
        assert!(untils[1] <= started + TALK_TIME + Duration::from_secs(1));
        assert!(untils[1] > started + Duration::from_secs(59));
        assert_same(
            short,
            Err(HostError::TimedOut(
                "stand-in wrote no line in time".to_string(),
            )),
        );
        assert!(matches!(long, Err(HostError::TimedOut(_))));
    }

    #[test]
    fn a_dropped_talk_stops_its_program_at_once() {
        let mut programs = programs(talking(&[]));

        let talk = programs
            .start_talk(
                "stand-in",
                &strings(&["serve", "--profile", "work"]),
                &[],
            )
            .expect("a talk");
        programs.drop_talk(talk);

        assert!(programs.send_line(talk, "x").is_err());
    }

    #[test]
    fn a_call_talks_to_at_most_four_programs_at_once() {
        let mut starter = MockStartProgram::new();
        starter.expect_start().times(RUNNING_CAP).returning(|_| {
            let mut program = MockRunningProgram::new();
            program.expect_stop().times(1).return_const(());
            Ok(Box::new(program))
        });
        let mut programs = programs(starter);
        let args = strings(&["serve", "--profile", "work"]);

        let talks: Vec<_> = (0..RUNNING_CAP)
            .map(|_| programs.start_talk("stand-in", &args, &[]))
            .collect();
        let one_more = programs.start_talk("stand-in", &args, &[]);
        drop(programs);

        assert!(talks.iter().all(Result::is_ok));
        assert!(matches!(one_more, Err(HostError::Failed(_))));
    }

    #[test]
    fn a_program_that_can_not_start_fails_in_plain_words() {
        let mut starter = MockStartProgram::new();
        starter.expect_start().with(always()).returning(|_| {
            Err(ProgramError::NotStarted("no such file".to_string()))
        });
        let mut programs = programs(starter);

        let talk = programs.start_talk(
            "stand-in",
            &strings(&["serve", "--profile", "work"]),
            &[],
        );

        assert_same(
            talk,
            Err(HostError::Failed(
                "stand-in: can not start it: no such file".to_string(),
            )),
        );
    }
}

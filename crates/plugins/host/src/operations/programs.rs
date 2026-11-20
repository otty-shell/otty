//! `programs`: a program run once, and a program talked to line by line,
//! served to the called part by its [`CallPrograms`].
//!
//! [`CallPrograms`]: crate::call_programs::CallPrograms

use std::time::{Duration, Instant};

use wasmtime::component::Resource;

use crate::call_programs::TalkHandle;
use crate::calls::CallState;
use crate::operations::v0_1::otty::host::programs::{self, Output};
use crate::operations::v0_1::otty::host::types::HostError;

impl programs::Host for CallState {
    fn run(
        &mut self,
        program: String,
        args: Vec<String>,
        env: Vec<(String, String)>,
        timeout_ms: u32,
    ) -> Result<Output, HostError> {
        let timeout = Duration::from_millis(u64::from(timeout_ms));
        let started = Instant::now();

        let output = self.programs_mut().run(&program, &args, &env, timeout);
        self.extend_deadline(started.elapsed());

        let (status, stdout, stderr) = output?.into_parts();
        Ok(Output {
            status,
            stdout,
            stderr,
        })
    }
}

impl programs::HostTalk for CallState {
    fn start(
        &mut self,
        program: String,
        args: Vec<String>,
        env: Vec<(String, String)>,
    ) -> Result<Resource<TalkHandle>, HostError> {
        let started = Instant::now();

        let talk = self.programs_mut().start_talk(&program, &args, &env);
        self.extend_deadline(started.elapsed());

        let talk = talk?;
        self.table().push(talk).map_err(|error| {
            self.programs_mut().drop_talk(talk);
            HostError::Failed(error.to_string())
        })
    }

    fn send_line(
        &mut self,
        talk: Resource<TalkHandle>,
        line: String,
    ) -> Result<(), HostError> {
        let talk = self.talk_handle(&talk)?;

        self.programs_mut().send_line(talk, &line)
    }

    fn next_line(
        &mut self,
        talk: Resource<TalkHandle>,
        timeout_ms: u32,
    ) -> Result<Option<String>, HostError> {
        let talk = self.talk_handle(&talk)?;
        let timeout = Duration::from_millis(u64::from(timeout_ms));
        let started = Instant::now();

        let line = self.programs_mut().next_line(talk, timeout);
        self.extend_deadline(started.elapsed());

        line
    }

    fn drop(&mut self, talk: Resource<TalkHandle>) -> wasmtime::Result<()> {
        let talk = self.table().delete(talk)?;
        self.programs_mut().drop_talk(talk);

        Ok(())
    }
}

impl CallState {
    /// The talk the plugin's resource `talk` stands for.
    fn talk_handle(
        &mut self,
        talk: &Resource<TalkHandle>,
    ) -> Result<TalkHandle, HostError> {
        self.table()
            .get(talk)
            .copied()
            .map_err(|error| HostError::Failed(error.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    use otty_plugin_manifest::PartRef;

    use crate::agent::{LoginRef, Profile};
    use crate::engine::PluginEngine;
    use crate::operations::v0_1::otty::host::types::{Limits, ReadError};
    use crate::programs::{
        LineOutput, MockRunningProgram, MockStartProgram, RunOutput,
    };
    use crate::testing::{TempDir, assert_same, load_programs_plugin};

    fn profile() -> Profile {
        Profile {
            agent: "test-agent".to_string(),
            folder: "/home/test-user/.test-agent".to_string(),
            is_default: true,
        }
    }

    /// The login whose variable names the way the test plugin reads the
    /// limits, such as `run`.
    fn way(name: &str) -> LoginRef {
        LoginRef::EnvVar(name.to_string())
    }

    /// The plan of `limits`, where the test plugin puts what the program
    /// answered.
    fn plan(limits: Result<Limits, ReadError>) -> Result<String, String> {
        limits
            .map(|limits| limits.plan.unwrap_or_default())
            .map_err(|error| format!("{error:?}"))
    }

    #[test]
    fn what_a_program_wrote_reaches_the_plugin() {
        let cache = TempDir::new();
        let mut starter = MockStartProgram::new();
        starter
            .expect_run()
            .withf(|command, _, _| {
                command.program() == "stand-in"
                    && command.args() == ["--version"]
                    && command.env()
                        == [(
                            "STAND_IN_HOME".to_string(),
                            "/home/test-user/.test-agent".to_string(),
                        )]
            })
            .times(1)
            .returning(|_, _, _| {
                Ok(RunOutput::new(Some(0), b"stand-in 1.2.3".to_vec(), vec![]))
            });
        let engine = PluginEngine::new(cache.path().to_path_buf())
            .expect("engine")
            .with_programs(Arc::new(starter));
        let plugin = load_programs_plugin(&engine);
        let mut batch = plugin
            .start_calls(&PartRef::agent("test-agent"))
            .expect("a listed part");

        let limits = batch
            .agent()
            .expect("an agent part")
            .read_limits(&profile(), &way("run"))
            .expect("the call");

        assert_same(plan(limits), Ok("stand-in 1.2.3".to_string()));
    }

    #[test]
    fn a_program_only_another_part_declares_is_refused_to_the_plugin() {
        let cache = TempDir::new();
        let mut starter = MockStartProgram::new();
        starter.expect_run().never();
        let engine = PluginEngine::new(cache.path().to_path_buf())
            .expect("engine")
            .with_programs(Arc::new(starter));
        let plugin = load_programs_plugin(&engine);
        let mut batch = plugin
            .start_calls(&PartRef::agent("other-agent"))
            .expect("a listed part");

        let limits = batch
            .agent()
            .expect("an agent part")
            .read_limits(&profile(), &way("run"))
            .expect("the call");

        assert_same(
            plan(limits),
            Err("ReadError::Host(HostError::NotAllowed(\"stand-in\"))"
                .to_string()),
        );
    }

    #[test]
    fn a_talk_the_plugin_keeps_ends_with_the_call() {
        let cache = TempDir::new();
        let stopped = Arc::new(AtomicBool::new(false));
        let stop = Arc::clone(&stopped);
        let mut program = MockRunningProgram::new();
        program
            .expect_send_line()
            .withf(|line| line == "ping")
            .returning(|_| Ok(()));
        program
            .expect_next_line()
            .times(1)
            .returning(|_| LineOutput::Line("pong".to_string()));
        program
            .expect_stop()
            .times(1)
            .returning(move || stop.store(true, Ordering::SeqCst));
        let mut starter = MockStartProgram::new();
        starter
            .expect_start()
            .times(1)
            .return_once(move |_| Ok(Box::new(program)));
        let engine = PluginEngine::new(cache.path().to_path_buf())
            .expect("engine")
            .with_programs(Arc::new(starter));
        let plugin = load_programs_plugin(&engine);
        let mut batch = plugin
            .start_calls(&PartRef::agent("test-agent"))
            .expect("a listed part");

        let talked = batch
            .agent()
            .expect("an agent part")
            .read_limits(&profile(), &way("talk"))
            .expect("the call");
        let stopped_with_the_call = stopped.load(Ordering::SeqCst);
        // Another profile, so the paced answer of the talk is not given.
        let other = Profile {
            folder: "/home/test-user/.test-agent-work".to_string(),
            is_default: false,
            ..profile()
        };
        let used_later = batch
            .agent()
            .expect("an agent part")
            .read_limits(&other, &way("used-talk"))
            .expect("the call");

        assert_same(plan(talked), Ok("pong".to_string()));
        assert!(stopped_with_the_call);
        assert_same(
            plan(used_later),
            Err(
                "ReadError::Host(HostError::Failed(\"the talk ended with the \
                 call that started it\"))"
                    .to_string(),
            ),
        );
    }
}

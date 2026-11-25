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

    use crate::engine::PluginEngine;
    use crate::programs::{
        LineOutput, MockRunningProgram, MockStartProgram, RunOutput,
    };
    use crate::testing::{
        TempDir, assert_same, load_programs_plugin, load_programs_plugin_with,
        plan_of, programs_logins_manifest, read_by_way, read_in_profiles,
    };

    /// The folder of the profile the account of the tests was found in.
    const FOLDER: &str = "/home/test-user/.test-agent";

    /// Issue 29: a provider part reads the numbers of an account by
    /// running the agent's program, under its own permissions.
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
                        == [("STAND_IN_HOME".to_string(), FOLDER.to_string())]
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
            .start_calls(&PartRef::provider("test-provider"))
            .expect("a listed part");

        let numbers = read_by_way(&mut batch, FOLDER, "run");

        assert_same(plan_of(numbers), Ok("stand-in 1.2.3".to_string()));
    }

    /// Spec 2026-11-23-accounts-apart-from-profiles: a part that names
    /// `STAND_IN_HOME` only as the `folder_env` of its logins starts
    /// `stand-in` pointed at a login folder of the call, never at a folder
    /// of an agent part its logins do not name, and in a call with no
    /// login folder not at all.
    #[test]
    fn a_program_gets_the_folder_variable_only_with_a_login_folder() {
        let cache = TempDir::new();
        let mut starter = MockStartProgram::new();
        starter
            .expect_run()
            .withf(|command, _, _| {
                command.env()
                    == [("STAND_IN_HOME".to_string(), FOLDER.to_string())]
            })
            .times(1)
            .returning(|_, _, _| {
                Ok(RunOutput::new(Some(0), b"stand-in 1.2.3".to_vec(), vec![]))
            });
        let engine = PluginEngine::new(cache.path().to_path_buf())
            .expect("engine")
            .with_programs(Arc::new(starter));
        let plugin =
            load_programs_plugin_with(&engine, &programs_logins_manifest());
        let mut batch = plugin
            .start_calls(&PartRef::provider("test-provider"))
            .expect("a listed part");
        let refused = "ReadError::Host(HostError::NotAllowed(\"variable \
                       STAND_IN_HOME\"))";

        let in_folder =
            read_in_profiles(&mut batch, &[("test-agent", FOLDER)], "run");
        let of_another_agent =
            read_in_profiles(&mut batch, &[("other-agent", FOLDER)], "run");
        let with_no_folder = read_in_profiles(&mut batch, &[], "run");

        assert_same(plan_of(in_folder), Ok("stand-in 1.2.3".to_string()));
        assert_same(plan_of(of_another_agent), Err(refused.to_string()));
        assert_same(plan_of(with_no_folder), Err(refused.to_string()));
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
            .start_calls(&PartRef::provider("other-provider"))
            .expect("a listed part");

        let numbers = read_by_way(&mut batch, FOLDER, "run");

        assert_same(
            plan_of(numbers),
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
            .start_calls(&PartRef::provider("test-provider"))
            .expect("a listed part");

        let talked = read_by_way(&mut batch, FOLDER, "talk");
        let stopped_with_the_call = stopped.load(Ordering::SeqCst);
        // Another profile, so the paced answer of the talk is not given.
        let used_later = read_by_way(
            &mut batch,
            "/home/test-user/.test-agent-work",
            "used-talk",
        );

        assert_same(plan_of(talked), Ok("pong".to_string()));
        assert!(stopped_with_the_call);
        assert_same(
            plan_of(used_later),
            Err(
                "ReadError::Host(HostError::Failed(\"the talk ended with the \
                 call that started it\"))"
                    .to_string(),
            ),
        );
    }

    /// A reading that ran a program runs once in five minutes per part
    /// and account; another account, and a reading that ran no program,
    /// read again.
    #[test]
    fn pacing_skips_a_second_reading_of_an_account_within_five_minutes() {
        let cache = TempDir::new();
        let mut starter = MockStartProgram::new();
        starter.expect_run().times(2).returning(|command, _, _| {
            let folder = command.env()[0].1.clone();
            Ok(RunOutput::new(Some(0), folder.into_bytes(), Vec::new()))
        });
        let engine = PluginEngine::new(cache.path().to_path_buf())
            .expect("engine")
            .with_programs(Arc::new(starter));
        let plugin = load_programs_plugin(&engine);
        let read = |folder: &str, way: &str| {
            let mut batch = plugin
                .start_calls(&PartRef::provider("test-provider"))
                .expect("a listed part");

            plan_of(read_by_way(&mut batch, folder, way)).ok()
        };
        let work = "/home/test-user/.test-agent-work";
        let quick = "/home/test-user/.test-agent-quick";

        let first = read(work, "run");
        let second = read(work, "run");
        let other_account = read(FOLDER, "run");
        let no_program = [read(quick, "none"), read(quick, "none")];

        assert_eq!(first.as_deref(), Some(work));
        assert_eq!(second, first);
        assert_eq!(other_account.as_deref(), Some(FOLDER));
        assert_eq!(
            no_program,
            [
                Some("no program".to_string()),
                Some("no program".to_string())
            ]
        );
    }
}

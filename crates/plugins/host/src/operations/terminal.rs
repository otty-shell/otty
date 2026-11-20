//! `terminal`: a program in a terminal nobody sees, served to the called
//! part by its [`CallTerminals`].
//!
//! [`CallTerminals`]: crate::call_terminals::CallTerminals

use std::time::{Duration, Instant};

use wasmtime::component::Resource;

use crate::call_terminals::TerminalHandle;
use crate::calls::CallState;
use crate::operations::v0_1::otty::host::terminal;
use crate::operations::v0_1::otty::host::types::HostError;

impl terminal::Host for CallState {}

impl terminal::HostHiddenTerminal for CallState {
    fn start(
        &mut self,
        program: String,
        args: Vec<String>,
        env: Vec<(String, String)>,
        cols: u16,
        rows: u16,
    ) -> Result<Resource<TerminalHandle>, HostError> {
        let started = Instant::now();

        let terminal = self
            .terminals_mut()
            .start(&program, &args, &env, cols, rows);
        self.extend_deadline(started.elapsed());

        let terminal = terminal?;
        self.table().push(terminal).map_err(|error| {
            self.terminals_mut().drop_terminal(terminal);
            HostError::Failed(error.to_string())
        })
    }

    fn send_keys(
        &mut self,
        terminal: Resource<TerminalHandle>,
        keys: String,
    ) -> Result<(), HostError> {
        let terminal = self.terminal_handle(&terminal)?;

        self.terminals_mut().send_keys(terminal, &keys)
    }

    fn screen(
        &mut self,
        terminal: Resource<TerminalHandle>,
    ) -> Result<String, HostError> {
        let terminal = self.terminal_handle(&terminal)?;

        self.terminals_mut().screen(terminal)
    }

    fn wait_for_change(
        &mut self,
        terminal: Resource<TerminalHandle>,
        timeout_ms: u32,
    ) -> Result<bool, HostError> {
        let terminal = self.terminal_handle(&terminal)?;
        let timeout = Duration::from_millis(u64::from(timeout_ms));
        let started = Instant::now();

        let changed = self.terminals_mut().wait_for_change(terminal, timeout);
        self.extend_deadline(started.elapsed());

        changed
    }

    fn drop(
        &mut self,
        terminal: Resource<TerminalHandle>,
    ) -> wasmtime::Result<()> {
        let terminal = self.table().delete(terminal)?;
        self.terminals_mut().drop_terminal(terminal);

        Ok(())
    }
}

impl CallState {
    /// The hidden terminal the plugin's resource `terminal` stands for.
    fn terminal_handle(
        &mut self,
        terminal: &Resource<TerminalHandle>,
    ) -> Result<TerminalHandle, HostError> {
        self.table()
            .get(terminal)
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
    use crate::terminal::{
        MockRunningTerminal, MockStartTerminal, ScreenOutput,
    };
    use crate::testing::{TempDir, load_programs_plugin};

    #[test]
    fn a_hidden_terminals_screen_text_reaches_the_plugin() {
        let cache = TempDir::new();
        let stopped = Arc::new(AtomicBool::new(false));
        let stop = Arc::clone(&stopped);
        let mut terminal = MockRunningTerminal::new();
        terminal
            .expect_send_keys()
            .withf(|keys| keys == "/usage\r")
            .times(1)
            .returning(|_| Ok(()));
        terminal.expect_next_screen().times(1).returning(|_| {
            ScreenOutput::Screen("Current session 29% used".to_string())
        });
        terminal
            .expect_stop()
            .times(1)
            .returning(move || stop.store(true, Ordering::SeqCst));
        let mut starter = MockStartTerminal::new();
        starter
            .expect_start()
            .withf(|command, cols, rows| {
                command.program() == "stand-in"
                    && command.args() == ["tui"]
                    && (*cols, *rows) == (100, 40)
            })
            .times(1)
            .return_once(move |_, _, _| Ok(Box::new(terminal)));
        let engine = PluginEngine::new(cache.path().to_path_buf())
            .expect("engine")
            .with_terminals(Arc::new(starter));
        let plugin = load_programs_plugin(&engine);
        let mut batch = plugin
            .start_calls(&PartRef::agent("test-agent"))
            .expect("a listed part");
        let profile = Profile {
            agent: "test-agent".to_string(),
            folder: "/home/test-user/.test-agent".to_string(),
            is_default: true,
        };

        let limits = batch
            .agent()
            .expect("an agent part")
            .read_limits(
                &profile,
                &LoginRef::EnvVar("STAND_IN_KEY".to_string()),
                "terminal",
            )
            .expect("the call");

        let plan = limits.ok().and_then(|limits| limits.plan);
        assert_eq!(plan.as_deref(), Some("Current session 29% used"));
        assert!(stopped.load(Ordering::SeqCst), "stopped with the call");
    }
}

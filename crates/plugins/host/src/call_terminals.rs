//! The hidden terminals of the running call: programs a part starts in a
//! terminal nobody sees to read their screen. Each is checked against the
//! permissions of the called part, kept to 60 s, and stopped when the
//! call that started it ends (spec "Host operations").

use std::sync::Arc;
use std::time::{Duration, Instant};

use otty_plugin_manifest::Permissions;

use crate::call_programs::{
    RUNNING_CAP, checked_command, free_slot, host_error,
};
use crate::operations::v0_1::otty::host::types::HostError;
use crate::programs::ProgramError;
use crate::terminal::{RunningTerminal, ScreenOutput, StartTerminal};

/// The longest a hidden terminal may run, from its start.
pub(crate) const TERMINAL_TIME: Duration = Duration::from_secs(60);
/// The widest hidden terminal, in columns.
const MAX_COLS: u16 = 500;
/// The tallest hidden terminal, in rows.
const MAX_ROWS: u16 = 200;

/// The host side of a `hidden-terminal` resource: which terminal of which
/// call of the batch. A handle kept past its call finds nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalHandle {
    call: u64,
    slot: usize,
}

/// The hidden terminals of the running call of one batch.
pub(crate) struct CallTerminals {
    starter: Option<Arc<dyn StartTerminal>>,
    permissions: Arc<Permissions>,
    call: u64,
    terminals: Vec<Option<Terminal>>,
    has_started: bool,
}

impl CallTerminals {
    /// Whether the running call has started a hidden terminal.
    pub(crate) fn has_started(&self) -> bool {
        self.has_started
    }

    /// The hidden terminals of the calls of a part with `permissions`,
    /// started by `starter`; with none, every terminal is refused.
    pub(crate) fn new(
        starter: Option<Arc<dyn StartTerminal>>,
        permissions: Arc<Permissions>,
    ) -> Self {
        Self {
            starter,
            permissions,
            call: 0,
            terminals: Vec::new(),
            has_started: false,
        }
    }

    /// A call begins: nothing has started in it yet.
    pub(crate) fn start_call(&mut self) {
        self.has_started = false;
    }

    /// The call ended: every terminal it started is stopped, and the
    /// handles it gave find nothing any more.
    pub(crate) fn end_call(&mut self) {
        self.stop_all();
        self.call += 1;
    }

    /// Starts `program` with `args` and the variables `env` in a hidden
    /// terminal of `cols` by `rows`, for at most 60 s.
    pub(crate) fn start(
        &mut self,
        program: &str,
        args: &[String],
        env: &[(String, String)],
        cols: u16,
        rows: u16,
    ) -> Result<TerminalHandle, HostError> {
        let may_start = self.permissions.may_start_terminal(program, args);
        let command =
            checked_command(&self.permissions, may_start, program, args, env)?;
        if !(1..=MAX_COLS).contains(&cols) || !(1..=MAX_ROWS).contains(&rows) {
            return Err(HostError::TooBig(format!(
                "a hidden terminal is 1 to {MAX_COLS} columns by 1 to \
                 {MAX_ROWS} rows, not {cols} by {rows}"
            )));
        }
        if self.terminals.iter().flatten().count() >= RUNNING_CAP {
            return Err(HostError::Failed(format!(
                "a call may keep at most {RUNNING_CAP} hidden terminals at \
                 once"
            )));
        }
        let starter = self.starter.clone().ok_or_else(no_starter)?;

        self.has_started = true;
        let running = starter
            .start(&command, cols, rows)
            .map_err(|error| host_error(program, &error))?;

        let terminal = Terminal {
            name: program.to_string(),
            program: running,
            screen: String::new(),
            deadline: Instant::now() + TERMINAL_TIME,
        };
        let slot = free_slot(&mut self.terminals);
        self.terminals[slot] = Some(terminal);

        Ok(TerminalHandle {
            call: self.call,
            slot,
        })
    }

    /// The working folder every hidden terminal starts in: as the app
    /// names it and, when links lead elsewhere, as the system resolves
    /// it. Refused to a part that declares no terminal.
    pub(crate) fn working_folders(&self) -> Result<Vec<String>, HostError> {
        if !self.permissions.has_terminals() {
            return Err(HostError::NotAllowed(
                "the working folder of hidden terminals".to_string(),
            ));
        }
        let starter = self.starter.as_ref().ok_or_else(no_starter)?;

        let folder = starter.working_folder();
        let mut folders = vec![folder.display().to_string()];
        if let Ok(real) = std::fs::canonicalize(&folder)
            && real != folder
        {
            folders.push(real.display().to_string());
        }

        Ok(folders)
    }

    /// Types `keys` into the program of `terminal`.
    pub(crate) fn send_keys(
        &mut self,
        terminal: TerminalHandle,
        keys: &str,
    ) -> Result<(), HostError> {
        self.expire_past(terminal, Instant::now())?;

        let terminal = self.terminal(terminal)?;
        terminal
            .program
            .send_keys(keys)
            .map_err(|error| host_error(&terminal.name, &error))
    }

    /// The visible text of the screen of `terminal`, as it was when it
    /// last changed; empty before the first change.
    pub(crate) fn screen(
        &mut self,
        terminal: TerminalHandle,
    ) -> Result<String, HostError> {
        let terminal = self.terminal(terminal)?;

        Ok(terminal.screen.clone())
    }

    /// Whether the screen of `terminal` changed within `timeout` and the
    /// rest of the terminal's 60 s. A program that ended fails, so a
    /// plugin does not wait for a screen that never comes; its last screen
    /// stays readable.
    pub(crate) fn wait_for_change(
        &mut self,
        terminal: TerminalHandle,
        timeout: Duration,
    ) -> Result<bool, HostError> {
        let now = Instant::now();
        self.expire_past(terminal, now)?;

        let terminal = self.terminal(terminal)?;
        let until = (now + timeout).min(terminal.deadline);
        match terminal.program.next_screen(until) {
            ScreenOutput::Screen(screen) => {
                terminal.screen = screen;
                Ok(true)
            },
            ScreenOutput::Unchanged => Ok(false),
            ScreenOutput::Ended => {
                Err(host_error(&terminal.name, &ProgramError::Ended))
            },
        }
    }

    /// The plugin dropped `terminal`: its program is stopped.
    pub(crate) fn drop_terminal(&mut self, terminal: TerminalHandle) {
        if terminal.call != self.call {
            return;
        }

        let running =
            self.terminals.get_mut(terminal.slot).and_then(Option::take);
        if let Some(mut running) = running {
            running.program.stop();
        }
    }

    /// The running terminal of `handle`, if its call still runs.
    fn terminal(
        &mut self,
        handle: TerminalHandle,
    ) -> Result<&mut Terminal, HostError> {
        let terminal = (handle.call == self.call)
            .then(|| self.terminals.get_mut(handle.slot))
            .flatten()
            .and_then(Option::as_mut);

        terminal.ok_or_else(|| {
            HostError::Failed(
                "the hidden terminal ended with the call that started it"
                    .to_string(),
            )
        })
    }

    /// Stops the terminal of `handle` once its 60 s have passed at `now`.
    fn expire_past(
        &mut self,
        handle: TerminalHandle,
        now: Instant,
    ) -> Result<(), HostError> {
        let terminal = self.terminal(handle)?;
        if now < terminal.deadline {
            return Ok(());
        }

        let name = terminal.name.clone();
        self.drop_terminal(handle);

        Err(HostError::TimedOut(format!(
            "the hidden terminal of {name} ran past {} s and was stopped",
            TERMINAL_TIME.as_secs()
        )))
    }

    fn stop_all(&mut self) {
        for mut terminal in self.terminals.drain(..).flatten() {
            terminal.program.stop();
        }
    }
}

impl Drop for CallTerminals {
    fn drop(&mut self) {
        self.stop_all();
    }
}

/// A program the running call started in a hidden terminal, and the
/// screen it showed last.
struct Terminal {
    name: String,
    program: Box<dyn RunningTerminal>,
    screen: String,
    deadline: Instant,
}

/// What a part that asks for a hidden terminal gets when the app gave the
/// host none.
fn no_starter() -> HostError {
    HostError::Failed(
        "this OTTY starts no hidden terminals for plugins".to_string(),
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use otty_plugin_manifest::Environment;

    use super::*;
    use crate::terminal::{MockRunningTerminal, MockStartTerminal};
    use crate::testing::{TempDir, assert_same, programs_permissions, strings};

    fn env() -> Environment {
        Environment::new("/home/test-user".into())
    }

    fn terminals(starter: MockStartTerminal) -> CallTerminals {
        CallTerminals::new(
            Some(Arc::new(starter)),
            programs_permissions("test-agent", &env()),
        )
    }

    /// A starter of one terminal that shows `screens` one after the other
    /// and is stopped once.
    fn showing(screens: &[&str]) -> MockStartTerminal {
        let screens = Mutex::new(strings(screens).into_iter());
        let mut terminal = MockRunningTerminal::new();
        terminal.expect_send_keys().returning(|_| Ok(()));
        terminal.expect_next_screen().returning(move |_| {
            let mut screens = screens.lock().expect("screens");
            screens
                .next()
                .map_or(ScreenOutput::Unchanged, ScreenOutput::Screen)
        });
        terminal.expect_stop().times(1).return_const(());

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
        starter
    }

    #[test]
    fn the_screen_text_reaches_the_plugin_after_each_change() {
        let mut terminals = terminals(showing(&["Loading", "Usage 29%"]));

        let terminal = terminals
            .start("stand-in", &strings(&["tui"]), &[], 100, 40)
            .expect("a terminal");
        let before = terminals.screen(terminal);
        let sent = terminals.send_keys(terminal, "/usage\r");
        let first = terminals.wait_for_change(terminal, Duration::from_secs(1));
        let loading = terminals.screen(terminal);
        let second =
            terminals.wait_for_change(terminal, Duration::from_secs(1));
        let usage = terminals.screen(terminal);
        let still = terminals.wait_for_change(terminal, Duration::from_secs(1));
        terminals.end_call();

        assert_same(before, Ok(String::new()));
        assert_same(sent, Ok(()));
        assert_same((first, second, still), (Ok(true), Ok(true), Ok(false)));
        assert_same(loading, Ok("Loading".to_string()));
        assert_same(usage, Ok("Usage 29%".to_string()));
        assert!(terminals.has_started());
    }

    #[test]
    fn an_undeclared_terminal_is_refused_and_nothing_starts() {
        let mut starter = MockStartTerminal::new();
        starter.expect_start().never();
        let mut terminals = terminals(starter);

        let other_args =
            terminals.start("stand-in", &strings(&["--yolo"]), &[], 100, 40);
        // `stand-in --version` is declared to run, not in a terminal.
        let run_only =
            terminals.start("stand-in", &strings(&["--version"]), &[], 100, 40);
        let unlisted_variable = terminals.start(
            "stand-in",
            &strings(&["tui"]),
            &[("HOME".to_string(), "/tmp".to_string())],
            100,
            40,
        );

        assert_same(
            other_args,
            Err(HostError::NotAllowed("stand-in".to_string())),
        );
        assert_same(
            run_only,
            Err(HostError::NotAllowed("stand-in".to_string())),
        );
        assert_same(
            unlisted_variable,
            Err(HostError::NotAllowed("variable HOME".to_string())),
        );
        assert!(!terminals.has_started());
    }

    #[test]
    fn a_terminal_of_no_size_or_a_huge_one_is_refused() {
        let mut starter = MockStartTerminal::new();
        starter.expect_start().never();
        let mut terminals = terminals(starter);
        let args = strings(&["tui"]);

        let empty = terminals.start("stand-in", &args, &[], 0, 40);
        let huge = terminals.start("stand-in", &args, &[], 100, u16::MAX);

        assert!(matches!(empty, Err(HostError::TooBig(_))));
        assert!(matches!(huge, Err(HostError::TooBig(_))));
    }

    #[test]
    fn a_terminal_ends_with_the_call() {
        let mut terminals = terminals(showing(&[]));
        terminals.start_call();
        let terminal = terminals
            .start("stand-in", &strings(&["tui"]), &[], 100, 40)
            .expect("a terminal");

        terminals.end_call();
        terminals.start_call();

        assert!(!terminals.has_started());
        assert_same(
            terminals.screen(terminal),
            Err(HostError::Failed(
                "the hidden terminal ended with the call that started it"
                    .to_string(),
            )),
        );
    }

    #[test]
    fn a_wait_keeps_to_the_terminals_sixty_seconds() {
        let untils = Arc::new(Mutex::new(Vec::new()));
        let seen = Arc::clone(&untils);
        let mut terminal = MockRunningTerminal::new();
        terminal.expect_next_screen().returning(move |until| {
            seen.lock().expect("untils").push(until);
            ScreenOutput::Unchanged
        });
        terminal.expect_stop().times(1).return_const(());
        let mut starter = MockStartTerminal::new();
        starter
            .expect_start()
            .return_once(move |_, _, _| Ok(Box::new(terminal)));
        let mut terminals = terminals(starter);
        let started = Instant::now();

        let handle = terminals
            .start("stand-in", &strings(&["tui"]), &[], 100, 40)
            .expect("a terminal");
        let changed =
            terminals.wait_for_change(handle, Duration::from_secs(600));
        drop(terminals);

        let untils = untils.lock().expect("untils");
        assert_same(changed, Ok(false));
        assert!(untils[0] <= started + TERMINAL_TIME + Duration::from_secs(1));
        assert!(untils[0] > started + Duration::from_secs(59));
    }

    #[test]
    fn a_program_that_ended_fails_the_wait_and_keeps_its_last_screen() {
        let mut terminal = MockRunningTerminal::new();
        let mut outputs =
            vec![ScreenOutput::Ended, ScreenOutput::Screen("bye".to_string())];
        terminal
            .expect_next_screen()
            .returning(move |_| outputs.pop().unwrap_or(ScreenOutput::Ended));
        terminal.expect_stop().times(1).return_const(());
        let mut starter = MockStartTerminal::new();
        starter
            .expect_start()
            .return_once(move |_, _, _| Ok(Box::new(terminal)));
        let mut terminals = terminals(starter);

        let handle = terminals
            .start("stand-in", &strings(&["tui"]), &[], 100, 40)
            .expect("a terminal");
        let changed = terminals.wait_for_change(handle, Duration::from_secs(1));
        let ended = terminals.wait_for_change(handle, Duration::from_secs(1));
        let last = terminals.screen(handle);
        terminals.end_call();

        assert_same(changed, Ok(true));
        assert_same(
            ended,
            Err(HostError::Failed("stand-in: it has ended".to_string())),
        );
        assert_same(last, Ok("bye".to_string()));
    }

    #[test]
    fn a_dropped_terminal_stops_its_program_at_once() {
        let mut terminals = terminals(showing(&[]));

        let terminal = terminals
            .start("stand-in", &strings(&["tui"]), &[], 100, 40)
            .expect("a terminal");
        terminals.drop_terminal(terminal);

        assert!(terminals.send_keys(terminal, "x").is_err());
    }

    /// Issue 13: a program names its records after the folder it ran in,
    /// so the part learns that folder, also as the system resolves it.
    #[cfg(unix)]
    #[test]
    fn the_working_folder_reaches_a_part_with_terminals() {
        let temp = TempDir::new();
        let real = temp.path().join("real-probe");
        std::fs::create_dir_all(&real).expect("the folder");
        let link = temp.path().join("agent-probe");
        std::os::unix::fs::symlink(&real, &link).expect("the link");
        let missing = temp.path().join("not-made-yet");
        let mut starter = MockStartTerminal::new();
        let mut folders = vec![missing.clone(), link.clone()].into_iter();
        starter
            .expect_working_folder()
            .times(2)
            .returning(move || folders.next().expect("a folder"));
        let terminals = terminals(starter);

        let not_made = terminals.working_folders();
        let linked = terminals.working_folders();

        let real = real.canonicalize().expect("the real folder");
        assert_same(not_made, Ok(vec![missing.display().to_string()]));
        assert_same(
            linked,
            Ok(vec![link.display().to_string(), real.display().to_string()]),
        );
    }

    #[test]
    fn a_part_with_no_terminal_does_not_learn_the_working_folder() {
        let mut starter = MockStartTerminal::new();
        starter.expect_working_folder().never();
        let terminals = CallTerminals::new(
            Some(Arc::new(starter)),
            programs_permissions("other-agent", &env()),
        );

        let folders = terminals.working_folders();

        assert_same(
            folders,
            Err(HostError::NotAllowed(
                "the working folder of hidden terminals".to_string(),
            )),
        );
    }

    #[test]
    fn no_starter_starts_no_terminal() {
        let mut terminals = CallTerminals::new(
            None,
            programs_permissions("test-agent", &env()),
        );

        let terminal =
            terminals.start("stand-in", &strings(&["tui"]), &[], 100, 40);

        assert!(matches!(terminal, Err(HostError::Failed(_))));
    }
}

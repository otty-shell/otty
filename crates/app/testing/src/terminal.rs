//! Terminals whose program is a mock, for tests.
//!
//! A terminal widget takes what it shows from the program behind it. A test
//! that started a real shell could not decide what the terminal shows: the
//! shell prints whenever it likes. Here the program is a [`mockall`] mock of
//! [`TerminalProgram`], and the test says which snapshot it has ready.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use mockall::mock;
use otty_ui_term::settings::Settings;
use otty_ui_term::{
    Error, ProgramEventSender, SnapshotOwned, StartedProgram, Terminal,
    TerminalProgram, TerminalRequest,
};

mock! {
    /// A mock of the program behind a terminal widget, for the tests of
    /// crates that cannot see the mock `otty-ui-term` keeps for its own.
    pub TerminalProgram {}

    impl TerminalProgram for TerminalProgram {
        fn send(&self, request: TerminalRequest);
        fn take_snapshot(&self) -> Option<SnapshotOwned>;
    }
}

/// What the terminals of a test show next, decided by the test.
///
/// A terminal started through [`ScriptedTerminals::starter`] has a mock in
/// place of its program. The mock takes any request and hands out the
/// snapshot the test reported for that terminal with
/// [`ScriptedTerminals::report`]; the terminal shows it on its next content
/// sync, as it would show the newest snapshot of a real program.
///
/// # Examples
///
/// ```
/// use otty_app_testing::terminal::ScriptedTerminals;
/// use otty_ui_term::settings::Settings;
/// use otty_ui_term::{Event, SnapshotOwned, Terminal};
///
/// let terminals = ScriptedTerminals::default();
/// let start = terminals.starter();
/// let mut terminal: Terminal =
///     start(7, Settings::default()).expect("a scripted terminal starts");
///
/// terminals.report(7, SnapshotOwned::default());
/// terminal.handle(Event::ContentSync { id: 7 });
/// ```
#[derive(Clone, Default)]
pub struct ScriptedTerminals {
    script: Arc<Mutex<Script>>,
}

impl ScriptedTerminals {
    /// Make the program of terminal `terminal_id` have `snapshot` ready. It
    /// replaces a reported snapshot the terminal did not show yet.
    pub fn report(&self, terminal_id: u64, snapshot: SnapshotOwned) {
        self.lock().ready.insert(terminal_id, snapshot);
    }

    /// Return how to start a terminal of this script: given the terminal's
    /// id and settings, it builds the terminal over a mock and starts no
    /// process.
    pub fn starter<T>(
        &self,
    ) -> impl Fn(u64, Settings) -> Result<Terminal<T>, Error> + 'static {
        let terminals = self.clone();

        move |terminal_id, settings| Ok(terminals.start(terminal_id, settings))
    }

    fn start<T>(&self, terminal_id: u64, settings: Settings) -> Terminal<T> {
        let program = self.program_of(terminal_id);
        let (started, events) = StartedProgram::new(Box::new(program));

        self.lock().events.push(events);

        Terminal::with_program(terminal_id, settings, started)
    }

    /// A mock that takes any request and hands out what was reported for
    /// terminal `terminal_id`.
    fn program_of(&self, terminal_id: u64) -> MockTerminalProgram {
        let terminals = self.clone();
        let mut program = MockTerminalProgram::new();
        program.expect_send().return_const(());
        program
            .expect_take_snapshot()
            .returning(move || terminals.lock().ready.remove(&terminal_id));

        program
    }

    fn lock(&self) -> MutexGuard<'_, Script> {
        // The script holds plain values, so it stays usable after a test
        // panicked while holding the lock.
        self.script.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[derive(Default)]
struct Script {
    /// The snapshot the program of each terminal has ready, by terminal id.
    ready: HashMap<u64, SnapshotOwned>,
    /// The event senders of the started terminals. No mock reports events;
    /// the senders are kept so the terminals' event channels stay open, as
    /// with a program that is still running.
    events: Vec<ProgramEventSender>,
}

#[cfg(test)]
mod tests {
    use otty_ui_term::settings::Settings;
    use otty_ui_term::{BlockSnapshot, Event, SnapshotOwned, Terminal};

    use super::*;

    /// A snapshot with `blocks` blocks, to tell snapshots apart.
    fn snapshot_with_blocks(blocks: usize) -> SnapshotOwned {
        let mut snapshot = SnapshotOwned::default();
        snapshot
            .blocks
            .extend((0..blocks).map(|_| BlockSnapshot::default()));

        snapshot
    }

    fn start(terminals: &ScriptedTerminals, id: u64) -> Terminal {
        (terminals.starter())(id, Settings::default())
            .expect("a scripted terminal always starts")
    }

    #[test]
    fn a_scripted_terminal_shows_the_reported_snapshot_on_its_next_content_sync()
     {
        let terminals = ScriptedTerminals::default();
        let mut terminal = start(&terminals, 7);

        terminals.report(7, snapshot_with_blocks(2));
        assert!(terminal.blocks().is_empty());
        terminal.handle(Event::ContentSync { id: 7 });

        assert_eq!(terminal.blocks().len(), 2);
    }

    #[test]
    fn a_reported_snapshot_is_shown_once_and_only_by_its_terminal() {
        let terminals = ScriptedTerminals::default();
        let mut first = start(&terminals, 1);
        let mut second = start(&terminals, 2);
        terminals.report(1, snapshot_with_blocks(1));

        second.handle(Event::ContentSync { id: 2 });
        first.handle(Event::ContentSync { id: 1 });
        // Nothing new is ready now, so the shown snapshot stays.
        first.handle(Event::ContentSync { id: 1 });

        assert!(second.blocks().is_empty());
        assert_eq!(first.blocks().len(), 1);
    }

    #[test]
    fn a_later_report_replaces_one_that_was_not_shown_yet() {
        let terminals = ScriptedTerminals::default();
        let mut terminal = start(&terminals, 7);
        terminals.report(7, snapshot_with_blocks(1));
        terminals.report(7, snapshot_with_blocks(3));

        terminal.handle(Event::ContentSync { id: 7 });

        assert_eq!(terminal.blocks().len(), 3);
    }

    #[test]
    fn a_scripted_terminal_takes_any_request() {
        let terminals = ScriptedTerminals::default();
        let mut terminal = start(&terminals, 7);

        terminal.handle(Event::Write {
            id: 7,
            data: b"ls\n".to_vec(),
        });

        // Dropping the terminal sends its shutdown request, also taken.
        drop(terminal);
    }
}

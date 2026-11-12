//! The program behind a terminal widget.
//!
//! A terminal widget shows a screen and takes input. The program whose
//! screen it is runs elsewhere: a shell behind a PTY on this computer or a
//! shell on another machine over SSH, each in threads of its own. This
//! module says what the widget needs from that program and starts the real
//! one.

use otty_libterm::surface::SnapshotOwned;
use otty_libterm::{
    DefaultParser, DefaultSurface, RuntimeRequestProxy, RuntimeTerminal,
    TerminalBuilder, TerminalEvent, TerminalHandle, TerminalRequest,
    TerminalSize, pty,
};
use tokio::sync::mpsc;

use crate::error::Result;
use crate::settings::{EngineSettings, SessionKind};

/// How many events of a started program may wait for its widget.
const EVENT_BUFFER: usize = 100;

type LocalTerminal =
    RuntimeTerminal<pty::LocalSession, DefaultParser, DefaultSurface>;

type SSHTerminal =
    RuntimeTerminal<pty::SSHSession, DefaultParser, DefaultSurface>;

/// The program behind a terminal widget: where the widget sends what the
/// user does and where it takes what to show.
///
/// It is a trait because the program is a process started through the
/// system, running in threads of its own. The widget talks to it only
/// through these two calls, so a test puts a mock here and runs without a
/// shell.
///
/// # Examples
///
/// ```
/// use otty_ui_term::{SnapshotOwned, TerminalProgram, TerminalRequest};
///
/// /// A program that ignores input and never has a new screen.
/// struct Idle;
///
/// impl TerminalProgram for Idle {
///     fn send(&self, _request: TerminalRequest) {}
///
///     fn take_snapshot(&self) -> Option<SnapshotOwned> {
///         None
///     }
/// }
///
/// assert!(Idle.take_snapshot().is_none());
/// ```
#[cfg_attr(test, mockall::automock)]
pub trait TerminalProgram {
    /// Pass a request of the widget to the program: input, a new size, a
    /// scroll, a selection, or the request to shut down.
    fn send(&self, request: TerminalRequest);

    /// Take the newest snapshot of the program's screen, if it has one that
    /// was not taken yet.
    fn take_snapshot(&self) -> Option<SnapshotOwned>;
}

/// The sender a started program reports its events to its widget through.
pub type ProgramEventSender = mpsc::Sender<TerminalEvent>;

/// What a started program gives its terminal widget: the program to talk
/// to and the events it reports.
///
/// # Examples
///
/// ```
/// use otty_ui_term::{
///     SnapshotOwned, StartedProgram, TerminalEvent, TerminalProgram,
///     TerminalRequest,
/// };
///
/// struct Idle;
///
/// impl TerminalProgram for Idle {
///     fn send(&self, _request: TerminalRequest) {}
///
///     fn take_snapshot(&self) -> Option<SnapshotOwned> {
///         None
///     }
/// }
///
/// let (_program, events) = StartedProgram::new(Box::new(Idle));
///
/// // Whoever runs the program reports its events through the sender.
/// events
///     .try_send(TerminalEvent::SnapshotReady)
///     .expect("the widget's side is still there");
/// ```
pub struct StartedProgram {
    program: Box<dyn TerminalProgram>,
    events: mpsc::Receiver<TerminalEvent>,
}

impl StartedProgram {
    /// Pair `program` with a new channel for its events.
    ///
    /// Returns the started program, for the widget, and the sender whoever
    /// runs the program reports its events through. The widget's
    /// subscription ends with an error when that sender is dropped before
    /// the program reported its exit, so keep it for as long as the program
    /// lives.
    pub fn new(
        program: Box<dyn TerminalProgram>,
    ) -> (Self, ProgramEventSender) {
        let (program_events, events) = mpsc::channel(EVENT_BUFFER);

        (Self { program, events }, program_events)
    }

    /// Split into the program and the receiver of its events.
    pub(crate) fn into_parts(
        self,
    ) -> (Box<dyn TerminalProgram>, mpsc::Receiver<TerminalEvent>) {
        (self.program, self.events)
    }
}

/// A program running behind an `otty-libterm` terminal, in threads of its
/// own.
struct RunningProgram {
    requests: RuntimeRequestProxy,
    handle: TerminalHandle,
}

impl TerminalProgram for RunningProgram {
    fn send(&self, request: TerminalRequest) {
        // The program may be gone already; there is nobody to tell then.
        let _ = self.requests.send(request);
    }

    fn take_snapshot(&self) -> Option<SnapshotOwned> {
        self.handle.take_snapshot()
    }
}

/// An `otty-libterm` terminal built for a session, not running yet.
enum SessionTerminal {
    Local(LocalTerminal),
    Ssh(SSHTerminal),
}

impl SessionTerminal {
    fn request_proxy(&self) -> RuntimeRequestProxy {
        match self {
            Self::Local((runtime, ..)) => runtime.proxy(),
            Self::Ssh((runtime, ..)) => runtime.proxy(),
        }
    }

    /// The handle the newest snapshot of the terminal is taken from.
    fn handle(&self) -> TerminalHandle {
        match self {
            Self::Local((_, _, handle, _)) => handle.clone(),
            Self::Ssh((_, _, handle, _)) => handle.clone(),
        }
    }

    /// Open the session `kind` asks for and build its terminal.
    fn build(kind: &SessionKind, size: TerminalSize) -> Result<Self> {
        match kind {
            SessionKind::Local(options) => {
                let mut builder = pty::local(options.program())
                    .with_args(options.args())
                    .with_size(size.into())
                    .set_controling_tty_enable();

                for (key, value) in options.envs() {
                    builder = builder.with_env(key, value);
                }

                if let Some(cwd) = options.working_directory() {
                    builder = builder.with_cwd(cwd)
                }

                let result =
                    TerminalBuilder::from(builder).build_with_runtime()?;

                Ok(Self::Local(result))
            },
            SessionKind::Ssh(options) => {
                let mut builder = pty::ssh()
                    .with_host(options.host())
                    .with_user(options.user())
                    .with_auth(options.auth())
                    .with_size(size.into());

                if let Some(timeout) = options.timeout() {
                    builder = builder.with_timeout(timeout);
                }

                if let Some(cancel) = options.cancel_token() {
                    builder = builder.with_cancel_token(cancel.clone());
                }

                let result =
                    TerminalBuilder::from(builder).build_with_runtime()?;

                Ok(Self::Ssh(result))
            },
        }
    }

    /// Pass the events of the terminal on to `widget_events`, in a thread
    /// of its own, until the program exits.
    fn forward_events(
        &self,
        widget_events: mpsc::Sender<TerminalEvent>,
    ) -> std::thread::JoinHandle<Result<()>> {
        let events = match self {
            Self::Local((.., events)) => events.clone(),
            Self::Ssh((.., events)) => events.clone(),
        };

        std::thread::spawn(move || {
            while let Ok(event) = events.recv() {
                let is_child_exit =
                    matches!(event, TerminalEvent::ChildExit { .. });
                let _ = widget_events.blocking_send(event);

                if is_child_exit {
                    break;
                }
            }

            Ok(())
        })
    }

    /// Run the terminal in a thread of its own.
    fn spawn(self) -> std::thread::JoinHandle<Result<()>> {
        std::thread::spawn(|| {
            match self {
                Self::Local((mut runtime, mut engine, ..)) => {
                    runtime.run(&mut engine, ())?
                },
                Self::Ssh((mut runtime, mut engine, ..)) => {
                    runtime.run(&mut engine, ())?
                },
            }

            Ok(())
        })
    }
}

/// Start the program `settings` ask for: open its session and run its
/// terminal in threads of their own.
pub(crate) fn start_program(
    settings: &EngineSettings,
) -> Result<StartedProgram> {
    let terminal = SessionTerminal::build(&settings.session, settings.size)?;
    let running = RunningProgram {
        requests: terminal.request_proxy(),
        handle: terminal.handle(),
    };
    let (started, widget_events) = StartedProgram::new(Box::new(running));

    let _ = terminal.forward_events(widget_events);
    let _ = terminal.spawn();

    Ok(started)
}

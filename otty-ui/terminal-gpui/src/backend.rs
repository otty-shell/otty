use std::path::{Path, PathBuf};
use std::process::ExitStatus;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use otty_libterm::pty::SSHAuth;
use otty_libterm::{
    ChannelConfig, DefaultParser, DefaultSurface, TerminalBuilder,
    TerminalEvents, TerminalHandle, TerminalSize, pty,
};

use crate::BackendError;

type BackendParts = (
    TerminalHandle,
    TerminalEvents,
    Box<dyn FnOnce() -> Result<(), BackendError> + Send>,
);

const TERMINAL_CHANNEL_CAPACITY: usize = 256;

/// Monotonically increasing identifier for one backend start attempt.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BackendGeneration(u64);

impl BackendGeneration {
    /// Numeric value useful for logging and host correlation.
    pub const fn value(self) -> u64 {
        self.0
    }

    /// Generation used for the first backend attached to a widget.
    pub const fn initial() -> Self {
        Self(1)
    }

    /// Return the following generation, saturating at the integer limit.
    pub const fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

/// Observable lifecycle state of the active terminal backend.
#[derive(Clone, Debug)]
pub enum BackendState {
    /// A backend is being constructed away from the UI thread.
    Starting,
    /// The terminal runtime is active.
    Running,
    /// Shutdown has been requested but the runtime has not exited yet.
    Stopping,
    /// The child process or remote command has exited.
    Exited(ExitStatus),
    /// Backend construction or runtime execution failed.
    Failed(Arc<BackendError>),
}

/// Started terminal channels plus the blocking runtime entry point.
pub struct BackendSession {
    handle: TerminalHandle,
    events: TerminalEvents,
    run: Box<dyn FnOnce() -> Result<(), BackendError> + Send>,
}

impl BackendSession {
    /// Create a session. The run callback is consumed exactly once by the widget.
    pub fn new(
        handle: TerminalHandle,
        events: TerminalEvents,
        run: impl FnOnce() -> Result<(), BackendError> + Send + 'static,
    ) -> Self {
        Self {
            handle,
            events,
            run: Box::new(run),
        }
    }

    pub(crate) fn into_parts(self) -> BackendParts {
        (self.handle, self.events, self.run)
    }
}

/// Infrastructure boundary used to start local, SSH, or custom terminals.
pub trait TerminalBackend: Send + 'static {
    /// Start the backend with the widget's latest effective grid size.
    fn start(
        self: Box<Self>,
        initial_size: TerminalSize,
    ) -> Result<BackendSession, BackendError>;
}

/// Options for launching a local PTY session.
#[derive(Clone, Debug)]
pub struct LocalOptions {
    program: String,
    args: Vec<String>,
    envs: Vec<(String, String)>,
    working_directory: Option<PathBuf>,
}

impl LocalOptions {
    /// Executable passed to the local PTY builder.
    pub fn program(&self) -> &str {
        &self.program
    }

    /// Command arguments in launch order.
    pub fn args(&self) -> &[String] {
        &self.args
    }

    /// Environment overrides as borrowed key/value pairs.
    pub fn envs(&self) -> Vec<(&str, &str)> {
        self.envs
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
            .collect()
    }

    /// Working directory, when configured.
    pub fn working_directory(&self) -> Option<&Path> {
        self.working_directory.as_deref()
    }

    /// Create options for the provided executable.
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            envs: Vec::new(),
            working_directory: None,
        }
    }

    /// Append one command argument.
    pub fn with_arg(mut self, argument: impl Into<String>) -> Self {
        self.args.push(argument.into());
        self
    }

    /// Add or replace one environment override.
    pub fn with_env(
        mut self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        let key = key.into();
        let value = value.into();
        if let Some(existing) =
            self.envs.iter_mut().find(|(existing, _)| existing == &key)
        {
            existing.1 = value;
        } else {
            self.envs.push((key, value));
        }
        self
    }

    /// Set the child process working directory.
    pub fn with_working_directory(mut self, directory: PathBuf) -> Self {
        self.working_directory = Some(directory);
        self
    }
}

impl Default for LocalOptions {
    fn default() -> Self {
        Self::new("/bin/sh")
    }
}

/// Built-in local PTY backend.
pub struct LocalBackend {
    options: LocalOptions,
}

impl LocalBackend {
    /// Borrow launch options.
    pub fn options(&self) -> &LocalOptions {
        &self.options
    }

    /// Create a local backend without starting a process.
    pub fn new(options: LocalOptions) -> Self {
        Self { options }
    }
}

impl TerminalBackend for LocalBackend {
    fn start(
        self: Box<Self>,
        initial_size: TerminalSize,
    ) -> Result<BackendSession, BackendError> {
        let mut builder = pty::local(self.options.program())
            .with_args(self.options.args())
            .with_size(initial_size.into())
            .set_controling_tty_enable();
        for (key, value) in &self.options.envs {
            builder = builder.with_env(key, value);
        }
        if let Some(directory) = self.options.working_directory() {
            builder = builder.with_cwd(directory);
        }

        let (mut runtime, mut engine, handle, events) = TerminalBuilder::<
            pty::LocalSession,
            DefaultParser,
            DefaultSurface,
        >::from(builder)
        .with_size(initial_size)
        .with_channel_config(ChannelConfig::bounded(TERMINAL_CHANNEL_CAPACITY))
        .build_with_runtime()?;

        Ok(BackendSession::new(handle, events, move || {
            runtime.run(&mut engine, ()).map_err(BackendError::from)
        }))
    }
}

/// Options for establishing an SSH terminal session.
#[derive(Clone, Debug)]
pub struct SshOptions {
    host: String,
    user: String,
    auth: SSHAuth,
    timeout: Option<Duration>,
    cancel_token: Option<Arc<AtomicBool>>,
}

impl SshOptions {
    /// Remote `host:port` pair.
    pub fn host(&self) -> &str {
        &self.host
    }

    /// Remote login user.
    pub fn user(&self) -> &str {
        &self.user
    }

    /// Authentication strategy.
    pub fn auth(&self) -> &SSHAuth {
        &self.auth
    }

    /// Connection timeout.
    pub fn timeout(&self) -> Option<Duration> {
        self.timeout
    }

    /// Cooperative cancellation flag.
    pub fn cancel_token(&self) -> Option<&Arc<AtomicBool>> {
        self.cancel_token.as_ref()
    }

    /// Create SSH options for a host, user, and authentication method.
    pub fn new(
        host: impl Into<String>,
        user: impl Into<String>,
        auth: SSHAuth,
    ) -> Self {
        Self {
            host: host.into(),
            user: user.into(),
            auth,
            timeout: None,
            cancel_token: None,
        }
    }

    /// Set a connection and authentication timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Set a cooperative cancellation flag.
    pub fn with_cancel_token(mut self, token: Arc<AtomicBool>) -> Self {
        self.cancel_token = Some(token);
        self
    }
}

/// Built-in SSH backend.
pub struct SshBackend {
    options: SshOptions,
}

impl SshBackend {
    /// Borrow connection options.
    pub fn options(&self) -> &SshOptions {
        &self.options
    }

    /// Create an SSH backend without connecting.
    pub fn new(options: SshOptions) -> Self {
        Self { options }
    }
}

impl TerminalBackend for SshBackend {
    fn start(
        self: Box<Self>,
        initial_size: TerminalSize,
    ) -> Result<BackendSession, BackendError> {
        let mut builder = pty::ssh()
            .with_host(self.options.host())
            .with_user(self.options.user())
            .with_auth(self.options.auth.clone())
            .with_size(initial_size.into());
        if let Some(timeout) = self.options.timeout() {
            builder = builder.with_timeout(timeout);
        }
        if let Some(token) = self.options.cancel_token() {
            builder = builder.with_cancel_token(Arc::clone(token));
        }

        let (mut runtime, mut engine, handle, events) = TerminalBuilder::<
            pty::SSHSession,
            DefaultParser,
            DefaultSurface,
        >::from(builder)
        .with_size(initial_size)
        .with_channel_config(ChannelConfig::bounded(TERMINAL_CHANNEL_CAPACITY))
        .build_with_runtime()?;

        Ok(BackendSession::new(handle, events, move || {
            runtime.run(&mut engine, ()).map_err(BackendError::from)
        }))
    }
}

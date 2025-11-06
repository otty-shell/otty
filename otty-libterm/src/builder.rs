#[cfg(unix)]
use std::{marker::PhantomData, path::Path};

#[cfg(unix)]
use crate::{
    error::Result,
    options::TerminalOptions,
    terminal::{Terminal, TerminalSurface},
};
#[cfg(unix)]
use otty_pty::{PtySize, UnixSessionBuilder, unix};
#[cfg(unix)]
use otty_surface::{Surface, SurfaceConfig};

/// Builder for launching a local Unix PTY session wrapped in the terminal runtime.
#[cfg(unix)]
pub struct UnixTerminalBuilder<S: TerminalSurface = Surface> {
    session: UnixSessionBuilder,
    surface_config: SurfaceConfig,
    options: TerminalOptions,
    _marker: PhantomData<S>,
}

#[cfg(unix)]
impl<S: TerminalSurface> UnixTerminalBuilder<S> {
    /// Start configuring a PTY session for the provided executable.
    #[must_use]
    pub fn new(program: &str) -> Self {
        Self {
            session: unix(program),
            surface_config: SurfaceConfig::default(),
            options: TerminalOptions::default(),
            _marker: PhantomData,
        }
    }

    /// Append a single argument to the spawned command.
    #[must_use]
    pub fn arg(mut self, arg: &str) -> Self {
        self.session = self.session.with_arg(arg);
        self
    }

    /// Append a slice of arguments to the spawned command.
    #[must_use]
    pub fn args(mut self, args: &[String]) -> Self {
        self.session = self.session.with_args(args);
        self
    }

    /// Set an environment variable for the spawned command.
    #[must_use]
    pub fn env(mut self, key: &str, value: &str) -> Self {
        self.session = self.session.with_env(key, value);
        self
    }

    /// Remove an environment variable from the spawned command.
    #[must_use]
    pub fn env_remove(mut self, key: &str) -> Self {
        self.session = self.session.with_env_remove(key);
        self
    }

    /// Configure the initial PTY size.
    #[must_use]
    pub fn size(mut self, size: PtySize) -> Self {
        self.session = self.session.with_size(size);
        self
    }

    /// Set the working directory for the child process.
    #[must_use]
    pub fn working_dir(mut self, path: &Path) -> Self {
        self.session = self.session.with_cwd(path);
        self
    }

    /// Request that the spawned process adopt the PTY as its controlling TTY.
    #[must_use]
    pub fn controlling_tty(mut self, enable: bool) -> Self {
        if enable {
            self.session = self.session.set_controling_tty_enable();
        }
        self
    }

    /// Override the surface parameters used to model terminal state.
    #[must_use]
    pub fn surface_config(mut self, config: SurfaceConfig) -> Self {
        self.surface_config = config;
        self
    }

    /// Override runtime options such as poll timeout and read buffer size.
    #[must_use]
    pub fn runtime_options(mut self, options: TerminalOptions) -> Self {
        self.options = options;
        self
    }

    /// Finalize the builder and spawn the terminal runtime using the default surface factory.
    pub fn spawn(self) -> Result<Terminal<S>> {
        let Self {
            session,
            surface_config,
            options,
            _marker,
        } = self;

        let session = session.spawn()?;
        Terminal::with_session(session, surface_config, options)
    }

    /// Finalize the builder and spawn the terminal runtime with a pre-built surface.
    pub fn spawn_with_surface(self, surface: S) -> Result<Terminal<S>> {
        let Self {
            session, options, ..
        } = self;

        let session = session.spawn()?;
        Terminal::with_session_and_surface(session, surface, options)
    }
}

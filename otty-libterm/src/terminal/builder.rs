use crate::terminal::TerminalEngine;
use crate::terminal::channel::ChannelConfig;
use crate::terminal::channel::{TerminalEvents, TerminalHandle};
use crate::terminal::options::TerminalOptions;
use crate::terminal::size::TerminalSize;
use crate::{Result, Runtime};
use crate::{
    escape::{self, EscapeParser},
    pty::{self, Pollable, Session},
    surface::{Surface, SurfaceActor, SurfaceConfig, SurfaceModel},
};

/// Default escape parser used by preset builders.
pub type DefaultParser = escape::Parser<escape::vte::Parser>;

/// Default surface used by preset builders.
pub type DefaultSurface = Surface;

/// Terminal emulator backend
pub type Terminal<P, E, S> =
    (TerminalEngine<P, E, S>, TerminalHandle, TerminalEvents);

/// Terminal emulator backend with runtime implementation
pub type RuntimeTerminal<P, E, S> = (
    Runtime,
    TerminalEngine<P, E, S>,
    TerminalHandle,
    TerminalEvents,
);

/// Builder that wires together a session, parser, and surface into a
/// [`TerminalEngine`].
pub struct TerminalBuilder<P, E, S> {
    session: SessionSource<P>,
    parser: E,
    surface: S,
    options: TerminalOptions,
    size: TerminalSize,
}

enum SessionSource<P> {
    Provided(Option<P>),
    Factory(Box<dyn FnMut(TerminalSize) -> Result<P> + Send>),
}

impl<P, E, S> TerminalBuilder<P, E, S>
where
    P: Session,
    S: SurfaceActor,
{
    /// Replace the session with a custom implementation.
    pub fn with_session<PS>(self, session: PS) -> TerminalBuilder<PS, E, S>
    where
        PS: Session,
    {
        TerminalBuilder {
            session: SessionSource::Provided(Some(session)),
            parser: self.parser,
            surface: self.surface,
            options: self.options,
            size: self.size,
        }
    }

    /// Replace the escape parser with a custom implementation.
    pub fn with_parser<EP>(self, parser: EP) -> TerminalBuilder<P, EP, S>
    where
        EP: EscapeParser,
    {
        TerminalBuilder {
            session: self.session,
            parser,
            surface: self.surface,
            options: self.options,
            size: self.size,
        }
    }

    /// Replace the surface with a custom implementation.
    pub fn with_surface<SA>(self, surface: SA) -> TerminalBuilder<P, E, SA>
    where
        SA: SurfaceActor,
    {
        TerminalBuilder {
            session: self.session,
            parser: self.parser,
            surface,
            options: self.options,
            size: self.size,
        }
    }

    /// Override the initial terminal geometry.
    pub fn with_size(mut self, size: TerminalSize) -> Self {
        self.size = size;
        self
    }

    /// Replace the terminal options (channel sizing, read buffer).
    pub fn with_options(mut self, options: TerminalOptions) -> Self {
        self.options = options;
        self
    }

    /// Override channel sizing for the request/event plumbing.
    pub fn with_channel_config(mut self, config: ChannelConfig) -> Self {
        self.options.channel_config = config;
        self
    }

    /// Override the temporary read buffer capacity used for PTY reads.
    pub fn with_read_buffer_capacity(mut self, capacity: usize) -> Self {
        self.options.read_buffer_capacity = capacity;
        self
    }
}

impl<P, E, S> TerminalBuilder<P, E, S>
where
    P: Session + Pollable,
    E: EscapeParser,
    S: SurfaceActor + SurfaceModel,
{
    /// Build a terminal engine, events receiver, and request handle.
    pub fn build(self) -> Result<Terminal<P, E, S>> {
        let TerminalBuilder {
            session,
            parser,
            surface,
            mut options,
            size,
        } = self;

        if options.read_buffer_capacity == 0 {
            options.read_buffer_capacity = 1024;
        }

        let session = spawn_session(session, size)?;

        let (mut engine, handle, events) =
            TerminalEngine::new(session, parser, surface, options)?;

        // Ensure the engine and surface start with the configured size.
        engine.resize(size)?;

        Ok((engine, handle, events))
    }

    /// Build a terminal engine bundle plus a mio runtime and proxy.
    pub fn build_with_runtime(self) -> Result<RuntimeTerminal<P, E, S>> {
        let runtime = Runtime::new()?;
        let (engine, handle, events) = self.build()?;
        Ok((runtime, engine, handle, events))
    }
}

impl From<pty::LocalSessionBuilder>
    for TerminalBuilder<pty::LocalSession, DefaultParser, DefaultSurface>
{
    fn from(builder: pty::LocalSessionBuilder) -> Self {
        let size = TerminalSize::default();
        Self {
            session: SessionSource::Factory(local_factory(builder)),
            parser: DefaultParser::default(),
            surface: DefaultSurface::new(SurfaceConfig::default(), &size),
            options: TerminalOptions::default(),
            size,
        }
    }
}

impl From<pty::SSHSessionBuilder>
    for TerminalBuilder<pty::SSHSession, DefaultParser, DefaultSurface>
{
    fn from(builder: pty::SSHSessionBuilder) -> Self {
        let size = TerminalSize::default();
        Self {
            session: SessionSource::Factory(ssh_factory(builder)),
            parser: DefaultParser::default(),
            surface: DefaultSurface::new(SurfaceConfig::default(), &size),
            options: TerminalOptions::default(),
            size,
        }
    }
}

impl<P, E> TerminalBuilder<P, E, DefaultSurface>
where
    P: Session,
{
    /// Override the surface configuration used by the default surface.
    pub fn with_surface_config(mut self, config: SurfaceConfig) -> Self {
        self.surface = DefaultSurface::new(config, &self.size);
        self
    }
}

fn spawn_session<P>(source: SessionSource<P>, size: TerminalSize) -> Result<P>
where
    P: Session,
{
    match source {
        SessionSource::Provided(mut session) => {
            session.take().ok_or_else(|| {
                crate::Error::Io(std::io::Error::other(
                    "session already consumed",
                ))
            })
        },
        SessionSource::Factory(mut factory) => factory(size),
    }
}

fn local_factory(
    builder: pty::LocalSessionBuilder,
) -> Box<dyn FnMut(TerminalSize) -> Result<pty::LocalSession> + Send> {
    let mut builder = Some(builder);
    Box::new(move |size: TerminalSize| {
        let builder = builder
            .take()
            .ok_or_else(|| {
                crate::Error::Io(std::io::Error::other(
                    "unix session builder already consumed",
                ))
            })?
            .with_size(size.into());
        builder.spawn().map_err(crate::Error::from)
    })
}

fn ssh_factory(
    builder: pty::SSHSessionBuilder,
) -> Box<dyn FnMut(TerminalSize) -> Result<pty::SSHSession> + Send> {
    let mut builder = Some(builder);
    Box::new(move |size: TerminalSize| {
        let builder = builder
            .take()
            .ok_or_else(|| {
                crate::Error::Io(std::io::Error::other(
                    "ssh session builder already consumed",
                ))
            })?
            .with_size(size.into());
        builder.spawn().map_err(crate::Error::from)
    })
}

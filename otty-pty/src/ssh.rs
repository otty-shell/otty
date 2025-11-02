//! SSH-based PTY backend that exposes remote shells through the shared
//! `Session` abstraction.

use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::os::unix::process::ExitStatusExt;
use std::path::Path;
use std::process::ExitStatus;

use mio::{Token, Waker};
use ssh2::{
    Channel, Error as SshError, ErrorCode, ExtendedData, Session as Ssh2Session,
};

use crate::{Pollable, PtySize, Session, SessionError};

const LIBSSH2_ERROR_EAGAIN: i32 = -37;
const REQUEST_PTY_TAG: &str = "xterm-256color";

/// Authentication strategy used when establishing an SSH session.
#[derive(Debug)]
pub enum SSHAuth {
    /// Authenticate with a plain-text password.
    Password(String),
    /// Authenticate with a private key file, optionally protected by a
    /// passphrase.
    KeyFile {
        private_key_path: String,
        passphrase: Option<String>,
    },
}

/// Interactive SSH session backed by `ssh2` crate
/// and integrated with Mio's poll loop.
pub struct SSHSession {
    _session: Ssh2Session,
    channel: Channel,

    io: mio::net::TcpStream,
    waker: Option<mio::Waker>,

    exit_status: Option<ExitStatus>,
    exit_notified: bool,
}

impl SSHSession {
    /// Construct a new SSH session wrapper with paired exit notification pipes.
    fn new(
        session: Ssh2Session,
        channel: Channel,
        io: mio::net::TcpStream,
    ) -> Self {
        Self {
            _session: session,
            channel,
            io,
            waker: None,
            exit_status: None,
            exit_notified: false,
        }
    }

    /// Notify the poller that the remote stream exited exactly once.
    fn notify_exit(&mut self) -> Result<(), SessionError> {
        if self.exit_notified {
            return Ok(());
        }

        if let Some(waker) = &self.waker {
            waker.wake()?;
        }

        self.exit_notified = true;
        Ok(())
    }

    /// Cache the remote exit status so repeated queries do not hit the network.
    fn try_cache_exit_status(
        &mut self,
    ) -> Result<Option<ExitStatus>, SessionError> {
        if let Some(status) = self.exit_status {
            return Ok(Some(status));
        }

        if !self.channel.eof() {
            return Ok(None);
        }

        match self.channel.exit_status() {
            Ok(code) => {
                let status = exit_status_from_code(code);
                self.exit_status = Some(status);
                Ok(Some(status))
            },
            Err(err) if is_would_block(&err) => Ok(None),
            Err(err) => Err(SessionError::SSH2(err)),
        }
    }
}

impl Session for SSHSession {
    /// Read from the SSH channel in non-blocking mode, emitting the bytes that
    /// arrive from the remote PTY.
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, SessionError> {
        match self.channel.read(buf) {
            Ok(0) => {
                let _ = self.try_cache_exit_status();
                self.notify_exit()?;
                Ok(0)
            },
            Ok(n) => Ok(n),
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => Ok(0),
            Err(e) => Err(SessionError::IO(e)),
        }
    }

    /// Forward bytes to the remote PTY, respecting libssh2's non-blocking
    /// semantics.
    fn write(&mut self, input: &[u8]) -> Result<usize, SessionError> {
        match self.channel.write(input) {
            Ok(n) => {
                let _ = self.channel.flush();
                Ok(n)
            },
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => Ok(0),
            Err(e) => Err(SessionError::IO(e)),
        }
    }

    /// Request a resize of the remote PTY dimensions, propagating both
    /// character and pixel sizes when provided.
    fn resize(&mut self, size: PtySize) -> Result<(), SessionError> {
        let pixel_width =
            (size.cell_width as u32).checked_mul(size.cols as u32);
        let pixel_height =
            (size.cell_height as u32).checked_mul(size.rows as u32);

        self.channel.request_pty_size(
            size.cols as u32,
            size.rows as u32,
            pixel_width,
            pixel_height,
        )?;

        Ok(())
    }

    /// Drive a graceful SSH channel teardown and surface the remote exit code
    /// when available.
    fn close(&mut self) -> Result<i32, SessionError> {
        for step in [
            Channel::send_eof,
            Channel::wait_eof,
            Channel::close,
            Channel::wait_close,
        ] {
            if let Err(err) = step(&mut self.channel) {
                if !is_would_block(&err) {
                    return Err(SessionError::SSH2(err));
                }
            }
        }

        let status = self
            .try_cache_exit_status()?
            .unwrap_or_else(|| exit_status_from_code(0));
        self.notify_exit()?;

        Ok(status.code().unwrap_or_default())
    }

    /// Poll the remote process for exit status without blocking on network I/O.
    fn try_get_child_exit_status(
        &mut self,
    ) -> Result<Option<ExitStatus>, SessionError> {
        let status = self.try_cache_exit_status()?;
        if status.is_some() {
            self.notify_exit()?;
        }
        Ok(status)
    }
}

impl Pollable for SSHSession {
    /// Register the libssh2 channel socket and the exit notifier pipe with Mio.
    fn register(
        &mut self,
        registry: &mio::Registry,
        interest: mio::Interest,
        io_token: Token,
        child_token: Token,
    ) -> Result<(), SessionError> {
        registry.register(&mut self.io, io_token, interest)?;
        self.waker = Some(Waker::new(registry, child_token)?);
        Ok(())
    }

    /// Update Mio's interest set for the SSH socket and exit notifier.
    fn reregister(
        &mut self,
        registry: &mio::Registry,
        interest: mio::Interest,
        io_token: Token,
        _: Token,
    ) -> Result<(), SessionError> {
        registry.reregister(&mut self.io, io_token, interest)?;
        Ok(())
    }

    /// Remove the SSH socket and exit notifier from the Mio registry.
    fn deregister(
        &mut self,
        registry: &mio::Registry,
    ) -> Result<(), SessionError> {
        registry.deregister(&mut self.io)?;
        let _ = self.waker.take();
        Ok(())
    }
}

impl Drop for SSHSession {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

impl Default for SSHAuth {
    fn default() -> Self {
        Self::Password(String::new())
    }
}

/// Builder that describes how to establish a new SSH-backed session.
#[derive(Debug, Default)]
pub struct SSHSessionBuilder {
    host: String,
    user: String,
    auth: SSHAuth,
    size: PtySize,
}

pub fn ssh() -> SSHSessionBuilder {
    SSHSessionBuilder::default()
}

impl SSHSessionBuilder {
    /// Set the `<host>:<port>` pair that the session should connect to.
    pub fn with_host(mut self, host: &str) -> Self {
        self.host = host.into();
        self
    }

    /// Set the user that should be authenticated on the remote host.
    pub fn with_user(mut self, user: &str) -> Self {
        self.user = user.into();
        self
    }

    /// Configure the authentication mechanism for the upcoming connection.
    pub fn with_auth(mut self, auth: SSHAuth) -> Self {
        self.auth = auth;
        self
    }

    /// Override the initial remote PTY size advertised to the server.
    pub fn with_size(mut self, size: PtySize) -> Self {
        self.size = size;
        self
    }

    /// Establish the SSH connection, negotiate a PTY, and return an interactive
    /// session that can be registered with Mio.
    pub fn spawn(self) -> Result<SSHSession, SessionError> {
        let SSHSessionBuilder {
            host,
            user,
            auth,
            size,
        } = self;

        let stream = TcpStream::connect(&host)?;
        stream.set_nonblocking(true)?;
        stream.set_nodelay(true)?;

        let mut session = Ssh2Session::new()?;
        session.set_tcp_stream(stream.try_clone()?);
        session.handshake()?;

        if let Ok(mut agent) = session.agent() {
            if agent.connect().is_ok() && agent.list_identities().is_ok() {
                for id in agent.identities().unwrap_or_default() {
                    if agent.userauth(&user, &id).is_ok() {
                        break;
                    }
                }
            }
        }

        if !session.authenticated() {
            match auth {
                SSHAuth::Password(pw) => {
                    session.userauth_password(&user, &pw)?;
                },
                SSHAuth::KeyFile {
                    private_key_path,
                    passphrase,
                } => {
                    let path = Path::new(&private_key_path);
                    session.userauth_pubkey_file(
                        &user,
                        None,
                        path,
                        passphrase.as_deref(),
                    )?;
                },
            }
        }

        let mut channel = session.channel_session()?;
        channel.handle_extended_data(ExtendedData::Merge)?;

        let pixel_width =
            (size.cell_width as u32).checked_mul(size.cols as u32);
        let pixel_height =
            (size.cell_height as u32).checked_mul(size.rows as u32);

        let pty_size = Some((
            size.cols as u32,
            size.rows as u32,
            pixel_width.unwrap_or(0),
            pixel_height.unwrap_or(0),
        ));

        channel.request_pty(REQUEST_PTY_TAG, None, pty_size)?;
        channel.shell()?;

        session.set_blocking(false);

        let mio_stream = mio::net::TcpStream::from_std(stream);
        mio_stream.set_nodelay(true)?;

        Ok(SSHSession::new(session, channel, mio_stream))
    }
}

/// Check whether a libssh2 error represents a non-blocking retry condition.
fn is_would_block(err: &SshError) -> bool {
    matches!(err.code(), ErrorCode::Session(code) if code == LIBSSH2_ERROR_EAGAIN)
}

/// Build an `ExitStatus` from the raw exit code reported by libssh2.
fn exit_status_from_code(code: i32) -> ExitStatus {
    ExitStatusExt::from_raw((code & 0xff) << 8)
}

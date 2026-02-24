//! SSH-based PTY backend that exposes remote shells through the shared
//! `Session` abstraction.

use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::path::Path;
use std::process::ExitStatus;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use log::debug;
use mio::{Events, Interest, Poll, Token, Waker};
use ssh2::{
    Channel, Error as SshError, ErrorCode, ExtendedData, Session as Ssh2Session,
};

use crate::{Pollable, PtySize, Session, SessionError};

const LIBSSH2_ERROR_EAGAIN: i32 = -37;
const REQUEST_PTY_TAG: &str = "xterm-256color";
// Upper bound for a single poll tick while connecting; keeps cancel/timeout
// checks responsive without busy-looping.
const CONNECT_POLL_MS: u64 = 200;
const SSH_RETRY_DELAY_MS: u64 = 10;

/// Authentication strategy used when establishing an SSH session.
#[derive(Debug, Clone)]
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
    fn try_get_exit_status(
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
            // If we receive EAGAIN it's not an error or exit status
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
            // Channel receive the EOF so we need to notify of exit
            Ok(0) => {
                let _ = self.try_get_exit_status();
                self.notify_exit()?;
                Ok(0)
            },
            Ok(n) => Ok(n),
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => Ok(0),
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
            .try_get_exit_status()?
            .unwrap_or_else(|| exit_status_from_code(0));
        self.notify_exit()?;

        Ok(status.code().unwrap_or_default())
    }

    /// Poll the remote process for exit status without blocking on network I/O.
    fn try_get_child_exit_status(
        &mut self,
    ) -> Result<Option<ExitStatus>, SessionError> {
        let status = self.try_get_exit_status()?;
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
    timeout: Option<Duration>,
    cancel: Option<Arc<AtomicBool>>,
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

    /// Set an overall timeout for connecting and authenticating the session.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Provide a cancellation token to abort session launch.
    pub fn with_cancel_token(mut self, cancel: Arc<AtomicBool>) -> Self {
        self.cancel = Some(cancel);
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
            timeout,
            cancel,
        } = self;

        let start = Instant::now();
        let cancel = cancel.as_ref();
        let executor = RetryableExecutor {
            start,
            timeout,
            cancel,
        };
        let stream = connect_with_timeout(&host, timeout, cancel)?;
        stream.set_nonblocking(true)?;
        stream.set_nodelay(true)?;

        let mut session = Ssh2Session::new()?;
        session.set_tcp_stream(stream.try_clone()?);
        session.set_blocking(false);
        executor.exec("ssh handshake", || session.handshake())?;

        if let Ok(mut agent) = session.agent() {
            if executor
                .exec("ssh agent connect", || agent.connect())
                .is_ok()
                && executor
                    .exec("ssh agent identities", || agent.list_identities())
                    .is_ok()
            {
                if let Ok(ids) =
                    executor.exec("ssh agent list", || agent.identities())
                {
                    for id in ids {
                        if executor
                            .exec("ssh agent auth", || {
                                agent.userauth(&user, &id)
                            })
                            .is_ok()
                        {
                            break;
                        }
                    }
                }
            }
        }

        if !session.authenticated() {
            match auth {
                SSHAuth::Password(pw) => {
                    executor.exec("ssh password auth", || {
                        session.userauth_password(&user, &pw)
                    })?;
                },
                SSHAuth::KeyFile {
                    private_key_path,
                    passphrase,
                } => {
                    let path = Path::new(&private_key_path);
                    executor.exec("ssh key auth", || {
                        session.userauth_pubkey_file(
                            &user,
                            None,
                            path,
                            passphrase.as_deref(),
                        )
                    })?;
                },
            }
        }

        let mut channel =
            executor.exec("ssh channel open", || session.channel_session())?;
        executor.exec("ssh channel setup", || {
            channel.handle_extended_data(ExtendedData::Merge)
        })?;

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

        executor.exec("ssh request pty", || {
            channel.request_pty(REQUEST_PTY_TAG, None, pty_size)
        })?;
        executor.exec("ssh shell", || channel.shell())?;

        let mio_stream = mio::net::TcpStream::from_std(stream);
        mio_stream.set_nodelay(true)?;

        Ok(SSHSession::new(session, channel, mio_stream))
    }
}

/// Check whether a libssh2 error represents a non-blocking retry condition.
fn is_would_block(err: &SshError) -> bool {
    matches!(err.code(), ErrorCode::Session(code) if code == LIBSSH2_ERROR_EAGAIN)
}

fn check_cancel(cancel: Option<&Arc<AtomicBool>>) -> Result<(), SessionError> {
    if let Some(cancel) = cancel {
        if cancel.load(Ordering::Relaxed) {
            return Err(SessionError::Cancelled);
        }
    }
    Ok(())
}

fn check_timeout(
    start: Instant,
    timeout: Option<Duration>,
    step: &'static str,
) -> Result<(), SessionError> {
    if let Some(timeout) = timeout {
        if start.elapsed() >= timeout {
            return Err(SessionError::Timeout {
                step,
                duration: timeout,
            });
        }
    }
    Ok(())
}

struct RetryableExecutor<'a> {
    start: Instant,
    timeout: Option<Duration>,
    cancel: Option<&'a Arc<AtomicBool>>,
}

impl<'a> RetryableExecutor<'a> {
    fn exec<T>(
        &self,
        step: &'static str,
        mut op: impl FnMut() -> Result<T, SshError>,
    ) -> Result<T, SessionError> {
        loop {
            check_cancel(self.cancel)?;
            check_timeout(self.start, self.timeout, step)?;
            match op() {
                Ok(result) => return Ok(result),
                Err(err) if is_would_block(&err) => {
                    std::thread::sleep(Duration::from_millis(
                        SSH_RETRY_DELAY_MS,
                    ));
                    continue;
                },
                Err(err) => return Err(SessionError::SSH2(err)),
            }
        }
    }
}

fn connect_with_timeout(
    host: &str,
    timeout: Option<Duration>,
    cancel: Option<&Arc<AtomicBool>>,
) -> Result<TcpStream, SessionError> {
    let start = Instant::now();
    // TODO: DNS lookup is blocking and not cancelable.
    let addrs: Vec<SocketAddr> = host.to_socket_addrs()?.collect();
    if addrs.is_empty() {
        return Err(SessionError::NoAddresses);
    }

    for addr in &addrs {
        match connect_addr_nonblocking(*addr, start, timeout, cancel) {
            Ok(stream) => return Ok(stream),
            Err(SessionError::IO(err)) => {
                debug!("ssh connect attempt to {addr} failed: {err}");
            },
            Err(SessionError::Timeout { .. }) => {
                return Err(SessionError::Timeout {
                    step: "tcp connect",
                    duration: timeout.unwrap_or_default(),
                });
            },
            Err(SessionError::Cancelled) => {
                return Err(SessionError::Cancelled);
            },
            Err(err) => return Err(err),
        }
    }

    Err(SessionError::Internal("ssh connect failed".to_string()))
}

/// Perform a non-blocking TCP connect driven by a local Mio poller.
///
/// Rationale:
/// - `connect_timeout` blocks and cannot be interrupted by a cancel token.
/// - A short poll tick lets us observe cancel/timeouts promptly while the OS
///   finishes the connect in the background.
///
/// Flow:
/// 1) Create a non-blocking socket and initiate `connect`.
/// 2) Wait for WRITABLE readiness; then `take_error()` to detect success/fail.
/// 3) Repeat with short poll ticks until success, cancel, or timeout.
fn connect_addr_nonblocking(
    addr: SocketAddr,
    start: Instant,
    timeout: Option<Duration>,
    cancel: Option<&Arc<AtomicBool>>,
) -> Result<TcpStream, SessionError> {
    let mut stream = mio::net::TcpStream::connect(addr)?;
    let mut poll = Poll::new()?;
    poll.registry()
        .register(&mut stream, Token(0), Interest::WRITABLE)?;
    let mut events = Events::with_capacity(4);

    loop {
        check_cancel(cancel)?;
        check_timeout(start, timeout, "tcp connect")?;

        // Use a short poll tick so cancel/timeout are observed promptly, but
        // never exceed the remaining overall timeout budget.
        let poll_timeout = timeout
            .map(|timeout| {
                let remaining = timeout.saturating_sub(start.elapsed());
                remaining.min(Duration::from_millis(CONNECT_POLL_MS))
            })
            .unwrap_or(Duration::from_millis(CONNECT_POLL_MS));

        poll.poll(&mut events, Some(poll_timeout))?;

        for event in events.iter() {
            if event.token() != Token(0) {
                continue;
            }

            if event.is_writable() || event.is_readable() {
                if let Some(err) = stream.take_error()? {
                    return Err(SessionError::IO(err));
                }
                poll.registry().deregister(&mut stream)?;
                return Ok(stream.into());
            }
        }
    }
}

/// Build an `ExitStatus` from the raw exit code reported by libssh2.
#[cfg(unix)]
fn exit_status_from_code(code: i32) -> ExitStatus {
    std::os::unix::process::ExitStatusExt::from_raw((code & 0xff) << 8)
}

/// Build an `ExitStatus` from the raw exit code reported by libssh2.
#[cfg(windows)]
fn exit_status_from_code(code: i32) -> ExitStatus {
    std::os::windows::process::ExitStatusExt::from_raw(code as u32)
}

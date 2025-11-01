use std::fs::File;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::os::fd::{AsRawFd, FromRawFd};
use std::path::Path;

use ssh2::{Channel, Session as Ssh2Session};

use crate::session::Session;
use crate::{PtySize, SessionError};

#[derive(Debug)]
pub enum SSHAuth {
    Password(String),
    KeyFile {
        private_key_path: String,
        passphrase: Option<String>,
    },
}

pub struct SSHSession {
    channel: Channel,
    raw: File,
}

impl Session for SSHSession {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, SessionError> {
        Ok(self.channel.read(buf)?)
    }

    fn write(&mut self, input: &[u8]) -> Result<usize, SessionError> {
        Ok(self.channel.write(input)?)
    }

    fn resize(&mut self, size: PtySize) -> Result<(), SessionError> {
        self.channel.request_pty_size(
            size.cols as u32,
            size.rows as u32,
            Some(size.cell_width as u32),
            Some(size.cell_height as u32),
        )?;
        Ok(())
    }

    fn close(&mut self) -> Result<i32, SessionError> {
        let _ = self.channel.send_eof();
        let _ = self.channel.wait_eof();
        let _ = self.channel.close();
        let _ = self.channel.wait_close();

        let code = self.channel.exit_status().unwrap_or(0);
        Ok(code)
    }

    fn try_wait(&mut self) -> Result<i32, SessionError> {
        let code = self.channel.exit_status()?;
        Ok(code)
    }

    fn raw(&self) -> &File {
        &self.raw
    }
}

impl Default for SSHAuth {
    fn default() -> Self {
        Self::Password("".into())
    }
}

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
    pub fn with_host(mut self, host: &str) -> Self {
        self.host = host.into();
        self
    }

    pub fn with_user(mut self, user: &str) -> Self {
        self.user = user.into();
        self
    }

    pub fn with_auth(mut self, auth: SSHAuth) -> Self {
        self.auth = auth;
        self
    }

    pub fn with_size(mut self, size: PtySize) -> Self {
        self.size = size;
        self
    }

    pub fn with_cwd<P>(self, _path: P) -> Self
    where
        P: AsRef<std::path::Path>,
    {
        self
    }

    pub fn with_env(self, _key: &str, _value: &str) -> Self {
        self
    }

    pub fn spawn(self) -> Result<impl Session, SessionError> {
        let stream = TcpStream::connect(&self.host)?;
        stream.set_nodelay(true)?;

        let raw_socket = stream.try_clone()?;

        let mut raw_session = Ssh2Session::new()?;
        raw_session.set_tcp_stream(raw_socket);
        raw_session.handshake()?;

        if let Ok(mut agent) = raw_session.agent() {
            if agent.connect().is_ok() && agent.list_identities().is_ok() {
                for id in agent.identities().unwrap_or_default() {
                    if agent.userauth(&self.user, &id).is_ok() {
                        break;
                    }
                }
            }
        }

        if !raw_session.authenticated() {
            match self.auth {
                SSHAuth::Password(ref pw) => {
                    raw_session.userauth_password(&self.user, pw)?;
                },
                SSHAuth::KeyFile {
                    ref private_key_path,
                    ref passphrase,
                } => {
                    let path = Path::new(private_key_path);
                    raw_session.userauth_pubkey_file(
                        &self.user,
                        None,
                        path,
                        passphrase.as_deref(),
                    )?;
                },
            }
        }

        let mut channel = raw_session.channel_session()?;
        channel.handle_extended_data(ssh2::ExtendedData::Merge)?;
        channel.request_pty(
            "xterm-256color",
            None,
            Some((
                self.size.cols as u32,
                self.size.rows as u32,
                self.size.cell_width as u32,
                self.size.cell_height as u32,
            )),
        )?;
        channel.shell()?;
        raw_session.set_blocking(false);

        let stream = unsafe { File::from_raw_fd(stream.as_raw_fd()) };

        Ok(SSHSession {
            channel,
            raw: stream,
        })
    }
}

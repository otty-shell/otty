use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use iced::Font;
use otty_libterm::TerminalSize;
use otty_libterm::pty::SSHAuth;

use crate::theme::ColorPalette;

#[cfg(target_os = "windows")]
const DEFAULT_SHELL: &str = "cmd.exe";

#[cfg(unix)]
const DEFAULT_SHELL: &str = "/bin/bash";

#[derive(Default, Clone)]
pub struct Settings {
    pub font: FontSettings,
    pub theme: ThemeSettings,
    pub backend: BackendSettings,
}

#[derive(Default, Debug, Clone)]
pub struct BackendSettings {
    pub session: SessionKind,
    pub size: TerminalSize,
}

impl BackendSettings {
    pub fn with_session(mut self, session: SessionKind) -> Self {
        self.session = session;
        self
    }

    pub fn with_size(mut self, size: TerminalSize) -> Self {
        self.size = size;
        self
    }
}

#[derive(Debug, Clone)]
pub struct LocalSessionOptions {
    program: String,
    args: Vec<String>,
    envs: HashMap<String, String>,
    working_directory: Option<PathBuf>,
}

impl Default for LocalSessionOptions {
    fn default() -> Self {
        Self {
            program: DEFAULT_SHELL.to_string(),
            args: vec![],
            envs: HashMap::from([
                ("TERM".to_string(), "xterm-256color".to_string()),
                ("COLORTERM".to_string(), "truecolor".to_string()),
            ]),
            working_directory: None,
        }
    }
}

impl LocalSessionOptions {
    pub fn with_program(mut self, program: &str) -> Self {
        self.program = program.to_string();
        self
    }

    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.args = args;
        self
    }

    pub fn with_envs(mut self, envs: HashMap<String, String>) -> Self {
        self.envs = envs;
        self
    }

    pub fn with_env(mut self, k: &str, v: &str) -> Self {
        self.envs.insert(k.to_string(), v.to_string());
        self
    }

    pub fn with_working_directory(mut self, dir: PathBuf) -> Self {
        self.working_directory = Some(dir);
        self
    }

    pub fn program(&self) -> &String {
        &self.program
    }

    pub fn args(&self) -> &[String] {
        &self.args
    }

    pub fn envs(&self) -> &HashMap<String, String> {
        &self.envs
    }

    pub fn working_directory(&self) -> &Option<PathBuf> {
        &self.working_directory
    }
}

#[derive(Default, Debug, Clone)]
pub struct SSHSessionOptions {
    host: String,
    user: String,
    auth: SSHAuth,
    timeout: Option<Duration>,
    cancel: Option<Arc<AtomicBool>>,
}

impl SSHSessionOptions {
    pub fn with_host(mut self, host: &str) -> Self {
        self.host = host.to_string();
        self
    }

    pub fn with_user(mut self, user: &str) -> Self {
        self.user = user.to_string();
        self
    }

    pub fn with_auth(mut self, auth: SSHAuth) -> Self {
        self.auth = auth;
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn with_cancel_token(mut self, cancel: Arc<AtomicBool>) -> Self {
        self.cancel = Some(cancel);
        self
    }

    pub fn host(&self) -> &String {
        &self.host
    }

    pub fn user(&self) -> &String {
        &self.user
    }

    pub fn auth(&self) -> SSHAuth {
        self.auth.clone()
    }

    pub fn timeout(&self) -> Option<Duration> {
        self.timeout
    }

    pub fn cancel_token(&self) -> Option<&Arc<AtomicBool>> {
        self.cancel.as_ref()
    }
}

#[derive(Debug, Clone)]
pub enum SessionKind {
    Local(LocalSessionOptions),
    Ssh(SSHSessionOptions),
}

impl Default for SessionKind {
    fn default() -> Self {
        Self::Local(LocalSessionOptions::default())
    }
}

impl SessionKind {
    pub fn from_local_options(options: LocalSessionOptions) -> Self {
        Self::Local(options)
    }

    pub fn from_ssh_options(options: SSHSessionOptions) -> Self {
        Self::Ssh(options)
    }
}

#[derive(Debug, Clone)]
pub struct FontSettings {
    pub size: f32,
    pub scale_factor: f32,
    pub font_type: Font,
}

impl Default for FontSettings {
    fn default() -> Self {
        Self {
            size: 14.0,
            scale_factor: 1.3,
            font_type: Font::MONOSPACE,
        }
    }
}

impl FontSettings {
    pub fn with_size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    pub fn with_scale_factor(mut self, scale_factor: f32) -> Self {
        self.scale_factor = scale_factor;
        self
    }

    pub fn with_font_type(mut self, font_type: Font) -> Self {
        self.font_type = font_type;
        self
    }
}

#[derive(Default, Debug, Clone)]
pub struct ThemeSettings {
    pub color_pallete: Box<ColorPalette>,
}

impl ThemeSettings {
    pub fn new(color_pallete: Box<ColorPalette>) -> Self {
        Self { color_pallete }
    }
}

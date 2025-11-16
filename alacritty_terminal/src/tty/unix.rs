//! TTY related functionality.

use std::ffi::CStr;
use std::io::{Error, Result};
use std::mem::MaybeUninit;
use std::{env, ptr};

use otty_pty::{PtySize, SSHSession, UnixSession, ssh, unix};
use crate::event::WindowSize;
use crate::tty::Options;

/// Create a new TTY and return a handle to interact with it.
pub fn new(
    config: Options,
    window_size: WindowSize,
    window_id: u64,
) -> Result<UnixSession> {
    let user = ShellUser::from_env()?;
    let mut builder = if let Some(shell) = config.shell.as_ref() {
        unix(&shell.program).with_args(shell.args.as_slice())
    } else {
        unix(&user.shell)
    };

    let pty_size = PtySize {
        rows: window_size.num_lines,
        cols: window_size.num_cols,
        cell_width: window_size.cell_width,
        cell_height: window_size.cell_height,
    };

    builder = builder
        .with_size(pty_size)
        .with_env("ALACRITTY_WINDOW_ID", &window_id.to_string())
        .with_env("USER", &user.user)
        .with_env("HOME", &user.home)
        .with_env("WINDOWID", &window_id.to_string())
        .set_controling_tty_enable()
        .with_env_remove("XDG_ACTIVATION_TOKEN")
        .with_env_remove("DESKTOP_STARTUP_ID");

    for (key, value) in &config.env {
        builder = builder.with_env(key, value);
    }

    if let Some(working_directory) = config.working_directory.as_ref() {
        builder = builder.with_cwd(working_directory);
    }

    let session = builder.spawn().map_err(Error::other)?;
    Ok(session)
}

/// Create a new TTY and return a handle to interact with it.
pub fn new_ssh(
    _: Options,
    window_size: WindowSize,
    _: u64,
) -> Result<SSHSession> {
    let pty_size = PtySize {
        rows: window_size.num_lines,
        cols: window_size.num_cols,
        cell_width: window_size.cell_width,
        cell_height: window_size.cell_height,
    };

    let builder = ssh()
        .with_size(pty_size)
        .with_host("192.168.0.160:22")
        .with_user("ilya-shvyryalkin")
        .with_auth(otty_pty::SSHAuth::Password("Tw7xro9uof__".into()));

    let session = builder.spawn().map_err(Error::other)?;
    Ok(session)
}

#[derive(Debug)]
struct Passwd<'a> {
    name: &'a str,
    dir: &'a str,
    shell: &'a str,
}

/// Return a Passwd struct with pointers into the provided buf.
///
/// # Unsafety
///
/// If `buf` is changed while `Passwd` is alive, bad thing will almost certainly happen.
fn get_pw_entry(buf: &mut [i8; 1024]) -> Result<Passwd<'_>> {
    // Create zeroed passwd struct.
    let mut entry: MaybeUninit<libc::passwd> = MaybeUninit::uninit();

    let mut res: *mut libc::passwd = ptr::null_mut();

    // Try and read the pw file.
    let uid = unsafe { libc::getuid() };
    let status = unsafe {
        libc::getpwuid_r(
            uid,
            entry.as_mut_ptr(),
            buf.as_mut_ptr() as *mut _,
            buf.len(),
            &mut res,
        )
    };
    let entry = unsafe { entry.assume_init() };

    if status < 0 {
        return Err(Error::other("getpwuid_r failed"));
    }

    if res.is_null() {
        return Err(Error::other("pw not found"));
    }

    // Sanity check.
    assert_eq!(entry.pw_uid, uid);

    // Build a borrowed Passwd struct.
    Ok(Passwd {
        name: unsafe { CStr::from_ptr(entry.pw_name).to_str().unwrap() },
        dir: unsafe { CStr::from_ptr(entry.pw_dir).to_str().unwrap() },
        shell: unsafe { CStr::from_ptr(entry.pw_shell).to_str().unwrap() },
    })
}

/// User information that is required for a new shell session.
struct ShellUser {
    user: String,
    home: String,
    shell: String,
}

impl ShellUser {
    /// look for shell, username, longname, and home dir in the respective environment variables
    /// before falling back on looking into `passwd`.
    fn from_env() -> Result<Self> {
        let mut buf = [0; 1024];
        let pw = get_pw_entry(&mut buf);

        let user = match env::var("USER") {
            Ok(user) => user,
            Err(_) => match pw {
                Ok(ref pw) => pw.name.to_owned(),
                Err(err) => return Err(err),
            },
        };

        let home = match env::var("HOME") {
            Ok(home) => home,
            Err(_) => match pw {
                Ok(ref pw) => pw.dir.to_owned(),
                Err(err) => return Err(err),
            },
        };

        let shell = match env::var("SHELL") {
            Ok(shell) => shell,
            Err(_) => match pw {
                Ok(ref pw) => pw.shell.to_owned(),
                Err(err) => return Err(err),
            },
        };

        Ok(Self { user, home, shell })
    }
}

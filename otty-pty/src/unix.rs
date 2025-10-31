use std::{io, os::fd::{FromRawFd, OwnedFd, RawFd}};

use nix::libc::{openpty as openpty_inner};

use crate::PtySize;

pub(crate) struct Pty {
    master: OwnedFd,
    slave: OwnedFd,
}

pub(crate) fn openpty(size: PtySize) -> std::io::Result<Pty> {
    let mut master: RawFd = -1;
    let mut slave: RawFd = -1;

    let result = unsafe {
        openpty_inner(
            &mut master,
            &mut slave,
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            &mut size.into()
        )
    };

    if result != 0 {
        return Err(io::Error::last_os_error());
    }

    let master_fd = unsafe { OwnedFd::from_raw_fd(master) };
    let slave_fd  = unsafe { OwnedFd::from_raw_fd(slave) };

    // let tty_name = unsafe { ttyname(slave) };

    // let master = UnixMasterPty {
    //     fd: ptyfd(unsafe { FileDescriptor::from_raw_fd(master) }),
    //     took_writer: RefCell::new(false),
    //     tty_name,
    // };
    // let slave = UnixSlavePty {
    //     fd: PtyFd(unsafe { FileDescriptor::from_raw_fd(slave) }),
    // };

    // Ensure that these descriptors will get closed when we execute
    // the child process.  This is done after constructing the Pty
    // instances so that we ensure that the Ptys get drop()'d if
    // the cloexec() functions fail (unlikely!).
    // cloexec(master.fd.as_raw_fd())?;
    // cloexec(slave.fd.as_raw_fd())?;

    // Ok((master, slave))

    Ok(Pty { master: master_fd, slave: slave_fd })
}